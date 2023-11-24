use crc32fast::Hasher;
use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Reset, Update};
use generic_array::typenum::U4;
use std::io::Write;

pub use digest::Digest;

// Updated from https://github.com/ajungren/crc32_digest/blob/master/src/lib.rs
// The original is Rust 2018 and doesnt seem to support the new digest crate

#[derive(Clone, Default)]
pub struct Crc32(Hasher);

#[allow(dead_code)]
impl Crc32 {
    /// Creates a new `Crc32`.
    #[inline]
    pub fn new() -> Self {
        Self(Hasher::new())
    }

    /// Creates a new `Crc32` initialized with the given state.
    #[inline]
    pub fn from_state(state: u32) -> Self {
        Self(Hasher::new_with_initial(state))
    }
}

impl HashMarker for Crc32 {}

impl Update for Crc32 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}

impl OutputSizeUser for Crc32 {
    type OutputSize = U4;
}

impl FixedOutput for Crc32 {
    #[inline]
    fn finalize_into(self, out: &mut Output<Self>) {
        // FixedOutput trait requires that the output is written into the given buffer of bytes
        // but crc32fast::Hasher::finalize() returns a u32, so we have to convert it
        let result = self.0.finalize();
        let r2 = result.to_le_bytes();
        out.copy_from_slice(&r2);
    }
}

impl Reset for Crc32 {
    #[inline]
    fn reset(&mut self) {
        self.0.reset();
    }
}

impl Write for Crc32 {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
