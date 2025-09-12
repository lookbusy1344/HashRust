use std::fmt;

use git_version::git_version;
use strum::EnumString;

pub const DEFAULT_HASH: HashAlgorithm = HashAlgorithm::SHA3_256;
pub const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
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
    #[strum(
        serialize = "SHA2",
        serialize = "SHA2-256",
        serialize = "SHA2_256",
        serialize = "SHA_256",
        serialize = "SHA-256"
    )]
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
pub enum OutputEncoding {
    Hex,
    Base64,
    Base32,
    U32,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BasicHash(pub String);

impl fmt::Display for BasicHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
