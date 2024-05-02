// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]

mod classes;
mod crc32;
mod hasher;
mod unit_tests;

use crate::classes::{
    BasicHash, ConfigSettings, HashAlgorithm, DEFAULT_HASH, GIT_VERSION_SHORT, HELP, VERSION,
};
//use crate::hasher::hash_file_crc32;
use blake2::{Blake2b512, Blake2s256};
use classes::OutputEncoding;
use glob::GlobResult;
use hasher::{file_exists, hash_file_encoded};
use md5::Md5;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};
use std::ffi::OsString;
use std::io;
use std::io::BufRead;
use std::str::FromStr;
use whirlpool::Whirlpool;

/// Call the inner worker function, and show help if there is an error
fn main() -> anyhow::Result<()> {
    let result = worker_func();

    if let Err(e) = result {
        // there was an error, show help
        show_help(true);
        println!();
        return Err(e);
    }

    Ok(())
}

/// main worker function for entire app
fn worker_func() -> anyhow::Result<()> {
    let mut pargs = pico_args::Arguments::from_env();

    // diagnostic code to set the parameters
    //let paramsvec: Vec<std::ffi::OsString> = vec!["--rubbish".into()];
    //println!("DIAGNOSTIC PARAMETERS SET: {paramsvec:?}");
    //let mut pargs = pico_args::Arguments::from_vec(paramsvec);

    if pargs.contains(["-h", "--help"]) {
        show_help(true);
        return Ok(());
    }

    // get algorithm as string and parse it
    let algostr: Option<String> = pargs.opt_value_from_str(["-a", "--algorithm"])?;
    let algo = parse_hash_algorithm(&algostr);

    if algo.is_err() {
        return Err(anyhow::anyhow!(
            "Algorithm can be: CRC32, MD5, SHA1, SHA2 / SHA2-256, SHA2-224, SHA2-384, SHA2-512, SHA3 / SHA3-256, SHA3-384, SHA3-512, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512. Default is {DEFAULT_HASH:?}",
        ));
    }

    // get output encoding as string and parse it
    let encodingstr: Option<String> = pargs.opt_value_from_str(["-e", "--encoding"])?;
    let encoding = parse_hash_encoding(&encodingstr);

    if encoding.is_err() {
        return Err(anyhow::anyhow!(
            "Encoding can be: Hex, Base64, Base32. Default is Hex",
        ));
    }

    // unwrap, and properly assign the default encoding
    let algo = algo.unwrap();
    let encoding = {
        let mut e = encoding.unwrap();

        if e == OutputEncoding::Unspecified {
            e = if algo == HashAlgorithm::CRC32 {
                OutputEncoding::U32 // default for CRC32
            } else {
                OutputEncoding::Hex // default for everything else
            };
        }

        // return the encoding, to be assigned to the variable
        e
    };

    // make sure CRC32 is only output as U32
    if algo == HashAlgorithm::CRC32 && encoding != OutputEncoding::U32 {
        return Err(anyhow::anyhow!("CRC32 can only be output as U32",));
    }

    // make sure other algorithms are not output as U32
    if algo != HashAlgorithm::CRC32 && encoding == OutputEncoding::U32 {
        return Err(anyhow::anyhow!(
            "This algorithm cannot be output as U32, please choose another encoding",
        ));
    }

    let config = ConfigSettings::new(
        pargs.contains(["-d", "--debug"]),
        pargs.contains(["-x", "--exclude-filenames"]),
        pargs.contains(["-s", "--single-thread"]),
        pargs.contains(["-c", "--case-sensitive"]),
        algo,
        encoding,
        pargs.opt_value_from_str(["-l", "--limit"])?,
    );

    // Check for unused arguments, and error out if there are any beginning with a dash
    // anything else might legitimately be a path, so we'll check that later
    let remainingargs = args_finished(pargs)?;

    // get the supplied path, if any. Turn the vector into a single Option<String>
    let suppliedpath = match remainingargs.len() {
        0 => None, // no path, we are expecting to read from stdin
        1 => Some(remainingargs[0].to_string_lossy().to_string()), // one path given
        _ => {
            // more than one path given, error out
            return Err(anyhow::anyhow!(
                "Only one path parameter can be given, but found {remainingargs:?}"
            ));
        }
    };

    if config.debugmode {
        show_help(false);
        eprintln!();
        eprintln!("Config: {config:?}");
        if suppliedpath.is_none() {
            eprintln!("No path specified, reading from stdin");
        } else {
            eprintln!("Path: {}", suppliedpath.as_ref().unwrap());
        }
    }

    // get the required files, either using supplied path or from reading stdin
    let mut paths = {
        if let Some(p) = suppliedpath {
            // path specified, use glob
            get_paths_matching_glob(&config, p.as_str())?
        } else {
            // no path specified, read from stdin
            get_paths_from_stdin(&config)?
        }
    };

    if config.limitnum.is_some() && paths.len() > config.limitnum.unwrap() {
        paths.truncate(config.limitnum.unwrap());
    }

    let paths = paths;
    if paths.is_empty() {
        if config.debugmode {
            eprintln!("No files found");
        }
        return Ok(());
    }

    if config.debugmode {
        eprintln!("Files to hash: {paths:?}");
    }

    if config.singlethread || paths.len() == 1 {
        // asked for single thread, or only one path given
        file_hashes_st(&config, &paths);
    } else {
        // multi-threaded
        file_hashes_mt(&config, &paths);
    }

    Ok(())
}

/// read from standard input and return a vector of strings
fn get_paths_from_stdin(config: &ConfigSettings) -> anyhow::Result<Vec<String>> {
    let stdin = io::stdin();
    let mut lines = Vec::with_capacity(20);

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                if file_exists(&line) {
                    lines.push(line);
                } else if config.debugmode {
                    eprintln!("Not a file: {line}");
                }
            }

            // problem reading line, I'm bailing out here and not just displaying an error
            Err(e) => return Err(e.into()), //eprintln!("read_stdin err {:?}", e),
        }
    }

    Ok(lines)
}

/// function to take a glob and return a vector of path strings
fn get_paths_matching_glob(config: &ConfigSettings, pattern: &str) -> anyhow::Result<Vec<String>> {
    let globsettings = glob::MatchOptions {
        case_sensitive: config.casesensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let temppaths = glob::glob_with(pattern, globsettings)?;

    // filter out non-files
    let pathglobs: Vec<GlobResult> = temppaths
        .filter(|x| x.as_ref().unwrap().is_file())
        .collect();

    // convert to vector of strings
    let paths: Vec<String> = pathglobs
        .into_iter()
        .map(|x| x.unwrap().to_string_lossy().to_string())
        .collect();

    Ok(paths)
}

/// output all file hashes matching a pattern, directly to stdout. Single-threaded
fn file_hashes_st(config: &ConfigSettings, paths: &[String]) {
    if config.debugmode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    for pathstr in paths {
        let filehash = call_hasher(config.algorithm, config.encoding, pathstr);

        match filehash {
            Ok(filehash) => {
                if config.excludefn {
                    println!("{}", filehash.0);
                } else {
                    println!("{} {}", filehash.0, pathstr);
                }
            }
            Err(e) => eprintln!("'{pathstr}' file err {e:?}"),
        }
    }
}

/// output all file hashes matching a pattern, directly to stdout. Multi-threaded version
fn file_hashes_mt(config: &ConfigSettings, paths: &[String]) {
    if config.debugmode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    // process the paths in parallel
    paths.par_iter().for_each(|pathstr| {
        let filehash = call_hasher(config.algorithm, config.encoding, pathstr);

        match filehash {
            Ok(hash) => {
                if config.excludefn {
                    println!("{}", hash.0);
                } else {
                    println!("{} {}", hash.0, pathstr);
                }
            }

            // failed to calculate the hash
            Err(e) => eprintln!("'{pathstr}' file err {e:?}"),
        }
    });
}

/// calculate the hash of a file using given algorithm
fn call_hasher(
    algo: HashAlgorithm,
    encoding: OutputEncoding,
    path: &str,
) -> anyhow::Result<BasicHash> {
    // panic if algo is CRC32 and output is not U32
    assert!(
        (algo == HashAlgorithm::CRC32 && encoding == OutputEncoding::U32)
            || (algo != HashAlgorithm::CRC32 && encoding != OutputEncoding::U32),
        "CRC32 can only be output as U32"
    );

    match algo {
        // special case, u32 encoded
        HashAlgorithm::CRC32 => hash_file_encoded::<crc32::Crc32>(path, OutputEncoding::U32),
        // old algorithms
        HashAlgorithm::MD5 => hash_file_encoded::<Md5>(path, encoding),
        HashAlgorithm::SHA1 => hash_file_encoded::<Sha1>(path, encoding),
        // SHA2
        HashAlgorithm::SHA2_224 => hash_file_encoded::<Sha224>(path, encoding),
        HashAlgorithm::SHA2_256 => hash_file_encoded::<Sha256>(path, encoding),
        HashAlgorithm::SHA2_384 => hash_file_encoded::<Sha384>(path, encoding),
        HashAlgorithm::SHA2_512 => hash_file_encoded::<Sha512>(path, encoding),
        // SHA3
        HashAlgorithm::SHA3_256 => hash_file_encoded::<Sha3_256>(path, encoding),
        HashAlgorithm::SHA3_384 => hash_file_encoded::<Sha3_384>(path, encoding),
        HashAlgorithm::SHA3_512 => hash_file_encoded::<Sha3_512>(path, encoding),
        // WHIRLPOOL
        HashAlgorithm::Whirlpool => hash_file_encoded::<Whirlpool>(path, encoding),
        // BLAKE2
        HashAlgorithm::Blake2S256 => hash_file_encoded::<Blake2s256>(path, encoding),
        HashAlgorithm::Blake2B512 => hash_file_encoded::<Blake2b512>(path, encoding),
    }
}

/// convert hash algorithm string into an enum
fn parse_hash_algorithm(algorithm: &Option<String>) -> Result<HashAlgorithm, strum::ParseError> {
    if algorithm.is_none() || algorithm.as_ref().unwrap().is_empty() {
        // no algorithm specified, use the default
        return Ok(DEFAULT_HASH);
    }

    HashAlgorithm::from_str(algorithm.as_ref().unwrap())
}

/// convert output encoding string into an enum
fn parse_hash_encoding(encoding: &Option<String>) -> Result<OutputEncoding, strum::ParseError> {
    if encoding.is_none() || encoding.as_ref().unwrap().is_empty() {
        // no algorithm specified, use the default
        return Ok(OutputEncoding::Unspecified);
    }

    OutputEncoding::from_str(encoding.as_ref().unwrap())
}

/// Show help message
fn show_help(longform: bool) {
    println!(
        "File hasher for various algorithms. Version {} ({})",
        VERSION.unwrap_or("?"),
        GIT_VERSION_SHORT
    );
    if longform {
        println!("{HELP}");
    }
    println!("Default algorithm is {DEFAULT_HASH:?}");
}

/// Check for unused arguments, and error out if there are any
fn args_finished(args: pico_args::Arguments) -> anyhow::Result<Vec<OsString>> {
    let unused = args.finish();

    // check any unused members do not start with a dash
    for arg in &unused {
        if arg.to_string_lossy().starts_with('-') {
            // this remaining argument starts with a dash, so it's an unknown argument
            return Err(anyhow::anyhow!(
                "Unknown argument: {}",
                arg.to_string_lossy()
            ));
        }
    }

    Ok(unused)
}
