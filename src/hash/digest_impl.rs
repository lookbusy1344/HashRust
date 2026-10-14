use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use data_encoding::{BASE32, BASE32HEX, BASE64};
use digest::{Digest, Output};

use crate::core::types::{BasicHash, OutputEncoding};

const BUFFER_SIZE: usize = 256 * 1024;

fn hash_file<D: Digest>(filename: impl AsRef<Path>) -> anyhow::Result<Output<D>> {
    Ok(hash_reader::<D>(File::open(filename)?)?)
}

/// Hashes `reader` to the end in `BUFFER_SIZE` chunks, retrying interrupted reads.
fn hash_reader<D: Digest>(mut reader: impl Read) -> io::Result<Output<D>> {
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut hasher = D::new();
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(hasher.finalize()),
            Ok(bytes_read) => hasher.update(&buffer[..bytes_read]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

#[inline]
pub fn hash_file_encoded<D: Digest>(
    filename: impl AsRef<Path>,
    encoding: OutputEncoding,
) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;

    Ok(BasicHash::new(match encoding {
        OutputEncoding::Hex => hex::encode(h),
        OutputEncoding::Base64 => BASE64.encode(&h),
        OutputEncoding::Base32 => BASE32.encode(&h),
        OutputEncoding::Base32Hex => BASE32HEX.encode(&h),
        OutputEncoding::U32 => {
            let bytes: [u8; 4] = h
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("When U32 is requested, hash size must be 4 bytes"))?;
            format!("{:010}", u32::from_be_bytes(bytes))
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::Md5;

    const MD5_OF_TEST: &str = "098f6bcd4621d373cade4e832627b4f6";

    /// Returns `error` before each byte of `data`, then one byte per read.
    struct FailingReader {
        data: &'static [u8],
        error: io::ErrorKind,
        fail_next: bool,
    }

    impl FailingReader {
        fn new(data: &'static [u8], error: io::ErrorKind) -> Self {
            Self {
                data,
                error,
                fail_next: true,
            }
        }
    }

    impl Read for FailingReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.data.is_empty() {
                return Ok(0);
            }
            if std::mem::take(&mut self.fail_next) {
                return Err(io::Error::from(self.error));
            }
            self.fail_next = true;
            buf[0] = self.data[0];
            self.data = &self.data[1..];
            Ok(1)
        }
    }

    #[test]
    fn test_hash_reader_retries_interrupted_reads() {
        let digest =
            hash_reader::<Md5>(FailingReader::new(b"test", io::ErrorKind::Interrupted)).unwrap();
        assert_eq!(hex::encode(digest), MD5_OF_TEST);
    }

    #[test]
    fn test_hash_reader_propagates_other_errors() {
        let err =
            hash_reader::<Md5>(FailingReader::new(b"test", io::ErrorKind::Other)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }
}
