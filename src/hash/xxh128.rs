use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Reset, Update};
use generic_array::typenum::U16;
use twox_hash::XxHash3_128;

/// XXH128 (XXH3, 128-bit, seed 0). Output is the canonical big-endian byte order.
#[derive(Clone)]
pub struct Xxh128(XxHash3_128);

impl Default for Xxh128 {
    fn default() -> Self {
        Self(XxHash3_128::new())
    }
}

impl HashMarker for Xxh128 {}

impl OutputSizeUser for Xxh128 {
    type OutputSize = U16;
}

impl Update for Xxh128 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.0.write(data);
    }
}

impl FixedOutput for Xxh128 {
    #[inline]
    fn finalize_into(self, out: &mut Output<Self>) {
        out.copy_from_slice(&self.0.finish_128().to_be_bytes());
    }
}

impl Reset for Xxh128 {
    #[inline]
    fn reset(&mut self) {
        self.0 = XxHash3_128::new();
    }
}
