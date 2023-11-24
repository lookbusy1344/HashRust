use crc32fast::Hasher;
use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Reset, Update};
use generic_array::typenum::U4;
//use generic_array::GenericArray;
use std::io::Write;

pub use digest::Digest;

#[derive(Clone, Default)]
pub struct Crc32(Hasher);

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
        let result = self.0.finalize();
        let r2 = result.to_le_bytes();
        out.copy_from_slice(&r2);
    }
}

// impl Input for Crc32 {
//     #[inline]
//     fn input<B: AsRef<[u8]>>(&mut self, data: B) {
//         self.0.update(data.as_ref());
//     }
// }

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

//impl_write!(Crc32);
