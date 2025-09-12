use std::io::Write;

use crc32fast::Hasher;
pub use digest::Digest;
use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Reset, Update};
use generic_array::typenum::U4;

#[derive(Clone, Default)]
pub struct Crc32(Hasher);

#[allow(dead_code)]
impl Crc32 {
    #[inline]
    pub fn new() -> Self {
        Self(Hasher::new())
    }

    #[inline]
    pub fn from_state(state: u32) -> Self {
        Self(Hasher::new_with_initial(state))
    }
}

impl HashMarker for Crc32 {}

impl OutputSizeUser for Crc32 {
    type OutputSize = U4;
}

impl Update for Crc32 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}

impl FixedOutput for Crc32 {
    #[inline]
    fn finalize_into(self, out: &mut Output<Self>) {
        let result = self.0.finalize();
        let r2 = result.to_be_bytes();
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