use std::ffi::OsString;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use pico_args::Arguments;

use crate::cli::config::{ConfigSettings, HELP};
use crate::core::types::{HashAlgorithm, OutputEncoding, DEFAULT_HASH, VERSION, GIT_VERSION_SHORT};

pub fn process_command_line(mut pargs: Arguments) -> Result<ConfigSettings> {
    let algo_str: Option<String> = pargs.opt_value_from_str(["-a", "--algorithm"])?;
    let algo = parse_hash_algorithm(algo_str.as_deref()).map_err(|_| {
        anyhow!(
        "Algorithm can be: CRC32, MD5, SHA1, SHA2 / SHA2-256 / SHA-256, SHA2-224, SHA2-384, SHA2-512, SHA3 / SHA3-256, SHA3-384, SHA3-512, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512. Default is {DEFAULT_HASH:?}",
    )
    })?;

    let encoding_str: Option<String> = pargs.opt_value_from_str(["-e", "--encoding"])?;
    let encoding = parse_hash_encoding(encoding_str.as_deref())
        .map_err(|_| anyhow!("Encoding can be: Hex, Base64, Base32. Default is Hex",))?;

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

    let mut config = ConfigSettings::new(
        pargs.contains(["-d", "--debug"]),
        pargs.contains(["-x", "--exclude-filenames"]),
        pargs.contains(["-s", "--single-thread"]),
        pargs.contains(["-c", "--case-sensitive"]),
        pargs.contains(["-n", "--no-progress"]),
        algo,
        encoding,
        pargs.opt_value_from_str(["-l", "--limit"])?,
    );

    let remaining_args = args_finished(pargs)?;

    let supplied_paths = remaining_args
        .into_iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();

    config.set_supplied_paths(supplied_paths);

    Ok(config)
}

fn parse_hash_algorithm(algorithm: Option<&str>) -> Result<HashAlgorithm, strum::ParseError> {
    match algorithm {
        Some(algo_str) if !algo_str.is_empty() => HashAlgorithm::from_str(algo_str),
        _ => Ok(DEFAULT_HASH),
    }
}

fn parse_hash_encoding(encoding: Option<&str>) -> Result<OutputEncoding, strum::ParseError> {
    match encoding {
        Some(enc_str) if !enc_str.is_empty() => OutputEncoding::from_str(enc_str),
        _ => Ok(OutputEncoding::Unspecified),
    }
}

pub fn show_help(longform: bool) {
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

fn args_finished(args: Arguments) -> Result<Vec<OsString>> {
    let unused = args.finish();

    for arg in &unused {
        if arg.to_string_lossy().starts_with('-') {
            return Err(anyhow!("Unknown argument: {}", arg.to_string_lossy()));
        }
    }

    Ok(unused)
}