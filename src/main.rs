// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]

mod hasher;

use crate::hasher::BasicHash;
use git_version::git_version;
use glob::GlobResult;
use hasher::{file_exists, hash_file};
use md5::Md5;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};
use std::ffi::OsString;
use std::io;
use std::io::BufRead;
use std::str::FromStr;
use strum::EnumString;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
enum HashAlgorithm {
    #[strum(serialize = "MD5", serialize = "MD-5")]
    MD5,
    #[strum(serialize = "SHA1", serialize = "SHA-1")]
    SHA1,
    #[strum(serialize = "SHA2", serialize = "SHA2-256", serialize = "SHA2_256")]
    SHA2_256,
    #[strum(serialize = "SHA2-384", serialize = "SHA2_384")]
    SHA2_384,
    #[strum(serialize = "SHA3", serialize = "SHA3-256", serialize = "SHA3_256")]
    SHA3_256,
    #[strum(serialize = "SHA2-512", serialize = "SHA2_512")]
    SHA2_512,
    #[strum(serialize = "SHA3-384", serialize = "SHA3_384")]
    SHA3_384,
    #[strum(serialize = "SHA3-512", serialize = "SHA3_512")]
    SHA3_512,
}

const DEFAULT_HASH: HashAlgorithm = HashAlgorithm::SHA3_256;
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const GIT_VERSION: &str = git_version!();

const HELP: &str = "\
USAGE:
    hash_rust.exe [flags] [options] file glob
FLAGS:
    -h, --help                   Prints help information
    -d, --debug                  Debug messages
    -c, --case-sensitive         Case-sensitive glob matching
    -x, --exclude-filenames      Exclude filenames from output
    -s, --single-thread          Single-threaded (not multi-threaded)
OPTIONS:
    -a, --algorithm [algorithm]  Hash algorithm to use
    -l, --limit [num]            Limit number of files processed
    
Algorithm can be:
    MD5, SHA1, 
    SHA2 / SHA2-256, SHA2-384, SHA2-512, 
    SHA3 / SHA3-256 (default), SHA3-384, SHA3-512";

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ConfigSettings {
    debugmode: bool,
    excludefn: bool,
    singlethread: bool,
    casesensitive: bool,
    algorithm: HashAlgorithm,
    limitnum: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct FileHash {
    path: String,
    hash: String,
}

/// Call the inner main function, and show help if there is an error
fn main() -> anyhow::Result<()> {
    let result = inner_main();

    if let Err(e) = result {
        // there was an error, show help
        show_help();
        println!();
        return Err(e);
    }

    Ok(())
}

/// Inner main function, which can return an error
fn inner_main() -> anyhow::Result<()> {
    let mut pargs = pico_args::Arguments::from_env();

    // diagnostic code to set the parameters
    //let paramsvec: Vec<std::ffi::OsString> = vec!["--rubbish".into()];
    //println!("DIAGNOSTIC PARAMETERS SET: {paramsvec:?}");
    //let mut pargs = pico_args::Arguments::from_vec(paramsvec);

    if pargs.contains(["-h", "--help"]) {
        show_help();
        return Ok(());
    }

    // get algorithm as string and parse it
    let algostr: Option<String> = pargs.opt_value_from_str(["-a", "--algorithm"])?;
    let algo = parse_hash_algorithm(&algostr);

    if algo.is_err() {
        return Err(anyhow::anyhow!(
            "Algorithm can be: MD5, SHA1, SHA2 / SHA2-256, SHA2-384, SHA2-512, SHA3 / SHA3-256, SHA3-384, SHA3-512. Default is {DEFAULT_HASH:?}",
        ));
    }

    let config = ConfigSettings {
        debugmode: pargs.contains(["-d", "--debug"]),
        excludefn: pargs.contains(["-x", "--exclude-filenames"]),
        singlethread: pargs.contains(["-s", "--single-thread"]),
        casesensitive: pargs.contains(["-c", "--case-sensitive"]),
        algorithm: algo.unwrap(),
        limitnum: pargs.opt_value_from_str(["-l", "--limit"])?,
    };

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
        if config.debugmode {
            eprintln!("Single-threaded mode");
        }
        file_hashes_st(&config, &paths);
    } else {
        if config.debugmode {
            eprintln!("Multi-threaded mode");
        }
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
                if file_exists(line.as_str()) {
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
        .into_iter()
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
fn file_hashes_st(config: &ConfigSettings, paths: &Vec<String>) {
    if config.debugmode {
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    for pathstr in paths {
        let filehash = call_hasher(config.algorithm, pathstr);

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
fn file_hashes_mt(config: &ConfigSettings, paths: &Vec<String>) {
    if config.debugmode {
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    // process the paths in parallel
    paths.par_iter().for_each(|pathstr| {
        let filehash = call_hasher(config.algorithm, pathstr);

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
fn call_hasher(algo: HashAlgorithm, path: &str) -> anyhow::Result<BasicHash> {
    match algo {
        // old algorithms
        HashAlgorithm::MD5 => hash_file::<Md5>(path),
        HashAlgorithm::SHA1 => hash_file::<Sha1>(path),
        // SHA2
        HashAlgorithm::SHA2_256 => hash_file::<Sha256>(path),
        HashAlgorithm::SHA2_384 => hash_file::<Sha384>(path),
        HashAlgorithm::SHA2_512 => hash_file::<Sha512>(path),
        // SHA3
        HashAlgorithm::SHA3_256 => hash_file::<Sha3_256>(path),
        HashAlgorithm::SHA3_384 => hash_file::<Sha3_384>(path),
        HashAlgorithm::SHA3_512 => hash_file::<Sha3_512>(path),
    }
}

/// convert hash algorithm string into an integer
fn parse_hash_algorithm(algorithm: &Option<String>) -> Result<HashAlgorithm, strum::ParseError> {
    if algorithm.is_none() || algorithm.as_ref().unwrap().is_empty() {
        // no algorithm specified, use the default
        return Ok(DEFAULT_HASH);
    }

    HashAlgorithm::from_str(algorithm.as_ref().unwrap())
}

/// Show help message
fn show_help() {
    println!(
        "File hasher for various algorithms. Version {} ({})",
        VERSION.unwrap_or("?"),
        GIT_VERSION
    );
    println!("{HELP}");
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
