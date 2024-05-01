use git_version::git_version;
use strum::EnumString;

pub const DEFAULT_HASH: HashAlgorithm = HashAlgorithm::SHA3_256;
pub const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
// pub const GIT_VERSION: &str = git_version!(args = ["--abbrev=40", "--always", "--dirty=+"]);
pub const GIT_VERSION_SHORT: &str = git_version!(args = ["--abbrev=14", "--always", "--dirty=+"]);

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum HashAlgorithm {
    #[strum(serialize = "CRC32", serialize = "CRC-32")]
    CRC32,
    #[strum(serialize = "MD5", serialize = "MD-5")]
    MD5,
    #[strum(serialize = "SHA1", serialize = "SHA-1")]
    SHA1,
    #[strum(serialize = "SHA2", serialize = "SHA2-256", serialize = "SHA2_256")]
    SHA2_256,
    #[strum(serialize = "SHA2-224", serialize = "SHA2_224")]
    SHA2_224,
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
    #[strum(serialize = "WHIRLPOOL")]
    Whirlpool,
    #[strum(serialize = "BLAKE2B-512", serialize = "BLAKE2B_512")]
    Blake2B512,
    #[strum(serialize = "BLAKE2S-256", serialize = "BLAKE2S_256")]
    Blake2S256,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ValueEncoding {
    Hex,
    Base64,
    Base32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Basic hash string. This is a wrapper around a String
pub struct BasicHash(pub String);

#[allow(clippy::struct_excessive_bools)]
#[readonly::make]
#[derive(Debug, Copy, Clone)]
/// Configuration settings
pub struct ConfigSettings {
    pub debugmode: bool,
    pub excludefn: bool,
    pub singlethread: bool,
    pub casesensitive: bool,
    pub algorithm: HashAlgorithm,
    pub limitnum: Option<usize>,
}

impl ConfigSettings {
    #[allow(clippy::fn_params_excessive_bools)]
    /// Create a new `ConfigSettings` object
    pub fn new(
        debugmode: bool,
        excludefn: bool,
        singlethread: bool,
        casesensitive: bool,
        algorithm: HashAlgorithm,
        limitnum: Option<usize>,
    ) -> Self {
        Self {
            debugmode,
            excludefn,
            singlethread,
            casesensitive,
            algorithm,
            limitnum,
        }
    }
}

pub const HELP: &str = "\
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
    CRC32, MD5, SHA1, WHIRLPOOL, BLAKE2S-256, BLAKE2B-512,
    SHA2 / SHA2-256, SHA-224, SHA2-384, SHA2-512, 
    SHA3 / SHA3-256 (default), SHA3-384, SHA3-512";
