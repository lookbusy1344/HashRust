pub mod types;
pub mod worker;

pub use types::{BasicHash, HashAlgorithm, OutputEncoding, DEFAULT_HASH, VERSION, GIT_VERSION_SHORT};
pub use worker::worker_func;