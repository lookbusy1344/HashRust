use crate::core::types::{HashAlgorithm, OutputEncoding};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct ConfigSettings {
    pub debug_mode: bool,
    pub exclude_fn: bool,
    pub single_thread: bool,
    pub case_sensitive: bool,
    pub no_progress: bool,
    pub algorithm: HashAlgorithm,
    pub encoding: OutputEncoding,
    pub limit_num: Option<usize>,
    pub supplied_paths: Vec<String>,
}

pub const HELP: &str = "\
USAGE:
    hash_rust [flags] [options] file glob
FLAGS:
    -h, --help                   Prints help information
    -d, --debug                  Debug messages
    -c, --case-sensitive         Case-sensitive glob matching
    -x, --exclude-filenames      Exclude filenames from output
    -s, --single-thread          Single-threaded (not multi-threaded)
    -n, --no-progress            Suppress progress display (for scripts)
OPTIONS:
    -a, --algorithm [algorithm]  Hash algorithm to use
    -e, --encoding [encoding]    Output encoding (Hex, Base64, Base32. Default is Hex)
    -l, --limit [num]            Limit number of files processed
    
Algorithm can be:
    CRC32, MD5, SHA1, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512,
    SHA2 / SHA2-256 / SHA-256, SHA2-224, SHA2-384, SHA2-512,
    SHA3 / SHA3-256 (default), SHA3-384, SHA3-512";
