// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]

use std::ffi::OsString;
use std::fmt::Display;
use std::io;
use std::io::BufRead;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};

//use crate::hasher::hash_file_crc32;
use blake2::{Blake2b512, Blake2s256};
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
    BasicHash, ConfigSettings, DEFAULT_HASH, GIT_VERSION_SHORT, HELP, HashAlgorithm, VERSION,
};

mod classes;
mod crc32;
mod hasher;
mod unit_tests;

/// Call the inner worker function, and show help if there is an error
fn main() -> Result<()> {
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
fn worker_func() -> Result<()> {
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
fn get_required_filenames(config: &ConfigSettings) -> Result<Vec<String>> {
    let mut paths = if config.supplied_paths.is_empty() {
        // No path specified, read from stdin
        get_paths_from_stdin(config)?
    } else {
        get_paths_matching_glob(config)?
    };

    // Limit the number of paths if required
    if let Some(limit) = config.limit_num {
        paths.truncate(limit);
    }

    Ok(paths)
}

fn show_initial_info(config: &ConfigSettings) {
    show_help(false);
    eprintln!();
    eprintln!("Config: {config:?}");
    if config.supplied_paths.is_empty() {
        eprintln!("No path specified, reading from stdin");
    } else {
        eprintln!(
            "Paths: {} file path(s) supplied",
            config.supplied_paths.len()
        );
    }
}

/// process the command line arguments and return a `ConfigSettings` struct
fn process_command_line(mut pargs: Arguments) -> Result<ConfigSettings> {
    // get algorithm as string and parse it
    let algo_str: Option<String> = pargs.opt_value_from_str(["-a", "--algorithm"])?;
    let algo = parse_hash_algorithm(algo_str.as_deref()).map_err(|_| {
        anyhow!(
        "Algorithm can be: CRC32, MD5, SHA1, SHA2 / SHA2-256 / SHA-256, SHA2-224, SHA2-384, SHA2-512, SHA3 / SHA3-256, SHA3-384, SHA3-512, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512. Default is {DEFAULT_HASH:?}",
    )
    })?;

    // get output encoding as string and parse it
    let encoding_str: Option<String> = pargs.opt_value_from_str(["-e", "--encoding"])?;
    let encoding = parse_hash_encoding(encoding_str.as_deref())
        .map_err(|_| anyhow!("Encoding can be: Hex, Base64, Base32. Default is Hex",))?;

    // properly assign the default encoding
    let encoding = match encoding {
        OutputEncoding::Unspecified if algo == HashAlgorithm::CRC32 => OutputEncoding::U32,
        OutputEncoding::Unspecified => OutputEncoding::Hex,
        other => other,
    };

    if algo == HashAlgorithm::CRC32 && encoding != OutputEncoding::U32 {
        return Err(anyhow!(
            "CRC32 must use U32 encoding, and U32 encoding can only be used with CRC32"
        ));
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
    // let supplied_path = match remaining_args.len() {
    //     0 => None, // no path, we are expecting to read from stdin
    //     1 => Some(remaining_args[0].to_string_lossy().to_string()), // one path given
    //     _ => {
    //         // more than one path given, error out
    //         return Err(anyhow!(
    //             "Only one path parameter can be given, but found {remaining_args:?}"
    //         ));
    //     }
    // };

    // get the supplied paths from remaining arguments
    let supplied_paths = remaining_args
        .into_iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();

    // add the supplied paths to config object
    config.set_supplied_paths(supplied_paths);

    Ok(config)
}

/// read from standard input and return a vector of strings
fn get_paths_from_stdin(config: &ConfigSettings) -> Result<Vec<String>> {
    let stdin = io::stdin();
    let lines = stdin.lock().lines().collect::<Result<Vec<String>, _>>()?;

    Ok(lines
        .into_iter()
        .filter(|line| {
            let is_file = file_exists(line);
            if !is_file && config.debug_mode {
                eprintln!("Not a file: {line}");
            }
            is_file
        })
        .collect())
}

/// function to take a glob and return a vector of path strings
fn get_paths_matching_glob(config: &ConfigSettings) -> Result<Vec<String>> {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut result = Vec::new();

    for pattern in &config.supplied_paths {
        // Try to match the pattern as a glob
        let glob_matches: Vec<_> = glob::glob_with(pattern, glob_settings)?
            .filter_map(|entry| match entry {
                Ok(path) if path.is_file() => Some(path.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        // If the glob matched nothing, check if the pattern itself is a valid file
        if glob_matches.is_empty() {
            if file_exists(pattern) {
                result.push(pattern.clone());
            } else {
                // Check if this looks like a specific file path (not a glob pattern)
                // If it doesn't contain glob metacharacters, treat it as a missing file error
                if !pattern.contains(&['*', '?', '[', ']']) {
                    return Err(anyhow::anyhow!("File not found: {}", pattern));
                }
                // Otherwise it's a glob pattern that matched nothing, which is acceptable
            }
        } else {
            result.extend(glob_matches);
        }
    }

    Ok(result)
}

/// output all file hashes matching a pattern, directly to stdout. Single-threaded
fn file_hashes_st<S>(config: &ConfigSettings, paths: &[S])
where
    S: AsRef<str> + Display,
{
    if config.debug_mode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    // Show progress bar for large file operations (10+ files)
    let progress_bar = if paths.len() >= 10 && !config.debug_mode {
        let pb = ProgressBar::new(paths.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .expect("Progress bar template should be valid")
                .progress_chars("##-"),
        );
        pb.set_message("Hashing files...");
        Some(pb)
    } else {
        None
    };

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
            Err(e) => eprintln!("File error for '{}': {}", pathstr, e),
        }

        if let Some(pb) = &progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Completed!");
    }
}

/// output all file hashes matching a pattern, directly to stdout. Multithreaded version
fn file_hashes_mt<S>(config: &ConfigSettings, paths: &[S])
where
    S: AsRef<str> + Sync + Display,
{
    if config.debug_mode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    // Show progress bar for large file operations (10+ files)
    let progress_bar = if paths.len() >= 10 && !config.debug_mode {
        let pb = ProgressBar::new(paths.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .expect("Progress bar template should be valid")
                .progress_chars("##-"),
        );
        pb.set_message("Hashing files...");
        Some(pb)
    } else {
        None
    };

    // process the paths in parallel
    paths.par_iter().for_each(|pathstr| {
        let file_hash = call_hasher(config.algorithm, config.encoding, pathstr);

        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }

            // failed to calculate the hash
            Err(e) => eprintln!("File error for '{}': {}", pathstr, e),
        }

        if let Some(pb) = &progress_bar {
            pb.inc(1);
        }
    });

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Completed!");
    }
}

/// calculate the hash of a file using given algorithm
fn call_hasher(
    algo: HashAlgorithm,
    encoding: OutputEncoding,
    path: impl AsRef<str>,
) -> Result<BasicHash> {
    // panic if algo is CRC32 and output is not U32
    if (algo == HashAlgorithm::CRC32 && encoding != OutputEncoding::U32)
        || (algo != HashAlgorithm::CRC32 && encoding == OutputEncoding::U32)
    {
        return Err(anyhow!(
            "CRC32 must use U32 encoding, and U32 encoding can only be used with CRC32"
        ));
    }

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

/// Convert hash algorithm string into an enum
fn parse_hash_algorithm(algorithm: Option<&str>) -> Result<HashAlgorithm, strum::ParseError> {
    match algorithm {
        Some(algo_str) if !algo_str.is_empty() => HashAlgorithm::from_str(algo_str),
        _ => Ok(DEFAULT_HASH),
    }
}

/// Convert output encoding string into an enum
fn parse_hash_encoding(encoding: Option<&str>) -> Result<OutputEncoding, strum::ParseError> {
    match encoding {
        Some(enc_str) if !enc_str.is_empty() => OutputEncoding::from_str(enc_str),
        _ => Ok(OutputEncoding::Unspecified),
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
fn args_finished(args: Arguments) -> Result<Vec<OsString>> {
    let unused = args.finish();

    // check any unused members do not start with a dash
    for arg in &unused {
        if arg.to_string_lossy().starts_with('-') {
            // this remaining argument starts with a dash, so it's an unknown argument
            return Err(anyhow!("Unknown argument: {}", arg.to_string_lossy()));
        }
    }

    Ok(unused)
}
