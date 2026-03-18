pub mod types;
pub mod worker;

pub(crate) use worker::FileHashError;
pub use worker::worker_func;
