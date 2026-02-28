use crc32fast::Hasher;
pub use digest::Digest;
use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Reset, Update};
use generic_array::typenum::U4;

#[derive(Clone, Default)]
pub struct Crc32(Hasher);

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
