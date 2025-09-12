use anyhow::{Result, anyhow};
use blake2::{Blake2b512, Blake2s256};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};
use whirlpool::Whirlpool;

use crate::core::types::{BasicHash, HashAlgorithm, OutputEncoding};
use crate::hash::crc32::Crc32;
use crate::hash::digest_impl::hash_file_encoded;

pub fn call_hasher(
    algo: HashAlgorithm,
    encoding: OutputEncoding,
    path: impl AsRef<str>,
) -> Result<BasicHash> {
    if (algo == HashAlgorithm::CRC32 && encoding != OutputEncoding::U32)
        || (algo != HashAlgorithm::CRC32 && encoding == OutputEncoding::U32)
    {
        return Err(anyhow!(
            "CRC32 must use U32 encoding, and U32 encoding can only be used with CRC32"
        ));
    }

    match algo {
        HashAlgorithm::CRC32 => hash_file_encoded::<Crc32>(path, OutputEncoding::U32),
        HashAlgorithm::MD5 => hash_file_encoded::<Md5>(path, encoding),
        HashAlgorithm::SHA1 => hash_file_encoded::<Sha1>(path, encoding),
        HashAlgorithm::SHA2_224 => hash_file_encoded::<Sha224>(path, encoding),
        HashAlgorithm::SHA2_256 => hash_file_encoded::<Sha256>(path, encoding),
        HashAlgorithm::SHA2_384 => hash_file_encoded::<Sha384>(path, encoding),
        HashAlgorithm::SHA2_512 => hash_file_encoded::<Sha512>(path, encoding),
        HashAlgorithm::SHA3_256 => hash_file_encoded::<Sha3_256>(path, encoding),
        HashAlgorithm::SHA3_384 => hash_file_encoded::<Sha3_384>(path, encoding),
        HashAlgorithm::SHA3_512 => hash_file_encoded::<Sha3_512>(path, encoding),
        HashAlgorithm::Whirlpool => hash_file_encoded::<Whirlpool>(path, encoding),
        HashAlgorithm::Blake2S256 => hash_file_encoded::<Blake2s256>(path, encoding),
        HashAlgorithm::Blake2B512 => hash_file_encoded::<Blake2b512>(path, encoding),
    }
}
