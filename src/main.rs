// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]

use std::ffi::OsString;
use std::io;
use std::io::BufRead;
use std::str::FromStr;

//use crate::hasher::hash_file_crc32;
use blake2::{Blake2b512, Blake2s256};
use glob::GlobResult;
use md5::Md5;
use pico_args::Arguments;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};
use whirlpool::Whirlpool;

use classes::OutputEncoding;
use hasher::{file_exists, hash_file_encoded};

use crate::classes::{
    BasicHash, ConfigSettings, DEFAULT_HASH, GIT_VERSION_SHORT, HashAlgorithm, HELP, VERSION,
};

mod classes;
mod crc32;
mod hasher;
mod unit_tests;

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
    let mut pargs = Arguments::from_env();

    // diagnostic code to set the parameters
    //let paramsvec: Vec<std::ffi::OsString> = vec!["--rubbish".into()];
    //println!("DIAGNOSTIC PARAMETERS SET: {paramsvec:?}");
    //let mut pargs = pico_args::Arguments::from_vec(paramsvec);

    // special handling of help
    if pargs.contains(["-h", "--help"]) {
        show_help(true);
        return Ok(());
    }

    // parse the command line arguments
    let config = process_command_line(pargs)?;

    if config.debug_mode {
        show_initial_info(&config);
    }

    // get the required files, either using supplied path or from reading stdin
    let paths = get_required_filenames(&config)?;

    if paths.is_empty() {
        if config.debug_mode {
            eprintln!("No files found");
        }
        return Ok(());
    }

    if config.debug_mode {
        eprintln!("Files to hash: {paths:?}");
    }

    if config.single_thread || paths.len() == 1 {
        // asked for single thread, or only one path given
        file_hashes_st(&config, &paths);
    } else {
        // multithreaded
        file_hashes_mt(&config, &paths);
    }

    Ok(())
}

/// get the required files, either using supplied path or from reading stdin
fn get_required_filenames(config: &ConfigSettings) -> anyhow::Result<Vec<String>> {
    let mut paths = if config.supplied_path.is_none() {
        // no path specified, read from stdin
        get_paths_from_stdin(config)?
    } else {
        get_paths_matching_glob(config)?
    };

    // limit the number of paths if required
    if config.limit_num.is_some() && paths.len() > config.limit_num.unwrap() {
        paths.truncate(config.limit_num.unwrap());
    }

    Ok(paths)
}

fn show_initial_info(config: &ConfigSettings) {
    show_help(false);
    eprintln!();
    eprintln!("Config: {config:?}");
    if config.supplied_path.is_none() {
        eprintln!("No path specified, reading from stdin");
    } else {
        eprintln!("Path: {}", config.supplied_path.as_ref().unwrap());
    }
}

/// process the command line arguments and return a `ConfigSettings` struct
fn process_command_line(mut pargs: Arguments) -> anyhow::Result<ConfigSettings> {
    // get algorithm as string and parse it
    let algo_str: Option<String> = pargs.opt_value_from_str(["-a", "--algorithm"])?;
    let algo = parse_hash_algorithm(&algo_str);

    if algo.is_err() {
        return Err(anyhow::anyhow!(
            "Algorithm can be: CRC32, MD5, SHA1, SHA2 / SHA2-256 / SHA-256, SHA2-224, SHA2-384, SHA2-512, SHA3 / SHA3-256, SHA3-384, SHA3-512, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512. Default is {DEFAULT_HASH:?}",
        ));
    }

    // get output encoding as string and parse it
    let encoding_str: Option<String> = pargs.opt_value_from_str(["-e", "--encoding"])?;
    let encoding = parse_hash_encoding(&encoding_str);

    if encoding.is_err() {
        return Err(anyhow::anyhow!(
            "Encoding can be: Hex, Base64, Base32. Default is Hex",
        ));
    }

    let algo = algo.unwrap(); // unwrap the algorithm

    // unwrap, and properly assign the default encoding
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
        panic!("CRC32 can only be output as U32");
    }

    // make sure other algorithms are not output as U32
    if algo != HashAlgorithm::CRC32 && encoding == OutputEncoding::U32 {
        panic!("This algorithm cannot be output as U32, please choose another encoding");
    }

    // build the config struct
    let mut config = ConfigSettings::new(
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
    let remaining_args = args_finished(pargs)?;

    // get the supplied path, if any. Turn the vector into a single Option<String>
    let supplied_path = match remaining_args.len() {
        0 => None, // no path, we are expecting to read from stdin
        1 => Some(remaining_args[0].to_string_lossy().to_string()), // one path given
        _ => {
            // more than one path given, error out
            return Err(anyhow::anyhow!(
                "Only one path parameter can be given, but found {remaining_args:?}"
            ));
        }
    };

    // add the supplied path to config object
    config.set_supplied_path(supplied_path);

    Ok(config)
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
                } else if config.debug_mode {
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
fn get_paths_matching_glob(config: &ConfigSettings) -> anyhow::Result<Vec<String>> {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    // we've already checked config.supplied_path is not None
    //assert!(config.supplied_path.is_some());

    // have to clone to unwrap the string, because the struct is borrowed
    let pattern = config.supplied_path.clone().unwrap();

    let temp_paths = glob::glob_with(&pattern, glob_settings)?;

    // filter out non-files
    let path_globs: Vec<GlobResult> = temp_paths
        .filter(|x| x.as_ref().unwrap().is_file())
        .collect();

    // convert to vector of strings
    let paths: Vec<String> = path_globs
        .into_iter()
        .map(|x| x.unwrap().to_string_lossy().to_string())
        .collect();

    Ok(paths)
}

/// output all file hashes matching a pattern, directly to stdout. Single-threaded
fn file_hashes_st(config: &ConfigSettings, paths: &[String]) {
    if config.debug_mode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    for pathstr in paths {
        let file_hash = call_hasher(config.algorithm, config.encoding, pathstr);

        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }
            Err(e) => eprintln!("'{pathstr}' file err {e:?}"),
        }
    }
}

/// output all file hashes matching a pattern, directly to stdout. Multithreaded version
fn file_hashes_mt(config: &ConfigSettings, paths: &[String]) {
    if config.debug_mode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    // process the paths in parallel
    paths.par_iter().for_each(|pathstr| {
        let file_hash = call_hasher(config.algorithm, config.encoding, pathstr);

        match file_hash {
            Ok(hash) => {
                if config.exclude_fn {
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
    match algorithm {
        Some(algo_str) if !algo_str.is_empty() => HashAlgorithm::from_str(algo_str), // parse the string
        _ => Ok(DEFAULT_HASH), // no algorithm specified (None, or empty string), use the default
    }
}

/// convert output encoding string into an enum
fn parse_hash_encoding(encoding: &Option<String>) -> Result<OutputEncoding, strum::ParseError> {
    match encoding {
        Some(enc_str) if !enc_str.is_empty() => OutputEncoding::from_str(enc_str), // parse the string
        _ => Ok(OutputEncoding::Unspecified), // no encoding specified (None, or empty string), use the default
    }
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
fn args_finished(args: Arguments) -> anyhow::Result<Vec<OsString>> {
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
