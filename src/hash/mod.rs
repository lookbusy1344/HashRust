pub mod algorithms;
pub mod crc32;
pub mod digest_impl;

pub use algorithms::call_hasher;
pub use digest_impl::hash_file_encoded;