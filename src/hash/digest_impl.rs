use std::fs::File;
use std::io::Read;
use std::path::Path;

use byteorder::{BigEndian, ByteOrder};
use data_encoding::{BASE32, BASE64};
use digest::{Digest, Output};

use crate::core::types::{BasicHash, OutputEncoding};

const BUFFER_SIZE: usize = 4096 * 8;

fn hash_file<D: Digest>(filename: impl AsRef<str>) -> anyhow::Result<Output<D>> {
    let filesize = usize::try_from(file_size(filename.as_ref())?).ok();

    if filesize.is_some_and(|size| size <= BUFFER_SIZE) {
        return hash_file_whole::<D>(filename);
    }

    let mut file = File::open(filename.as_ref())?;
    // 32KB is well within typical stack limits (2-8MB) and avoids heap allocation overhead
    #[allow(clippy::large_stack_arrays)]
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut hasher = D::new();

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

fn hash_file_whole<D: Digest>(filename: impl AsRef<str>) -> anyhow::Result<Output<D>> {
    let data = std::fs::read(filename.as_ref())?;
    Ok(D::digest(&data))
}

#[inline]
pub fn hash_file_encoded<D: Digest>(
    filename: impl AsRef<str>,
    encoding: OutputEncoding,
) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;

    Ok(BasicHash(match encoding {
        OutputEncoding::Hex => hex::encode(h),
        OutputEncoding::Base64 => BASE64.encode(&h),
        OutputEncoding::Base32 => BASE32.encode(&h),
        OutputEncoding::U32 => {
            if h.len() != 4 {
                return Err(anyhow::anyhow!(
                    "When U32 is requested, hash size must be 4 bytes"
                ));
            }

            let number = BigEndian::read_u32(&h);
            format!("{number:010}")
        }
    }))
}

fn file_size(path: impl AsRef<str>) -> anyhow::Result<u64> {
    let path = Path::new(path.as_ref());
    Ok(path.metadata()?.len())
}
