use std::fs::File;
use std::io::Read;
use std::path::Path;

use data_encoding::{BASE32, BASE32HEX, BASE64};
use digest::{Digest, Output};

use crate::core::types::{BasicHash, OutputEncoding};

const BUFFER_SIZE: usize = 4096 * 8;

fn hash_file<D: Digest>(filename: impl AsRef<Path>) -> anyhow::Result<Output<D>> {
    let mut file = File::open(filename)?;
    // usize::try_from can only fail on 32-bit targets where usize < u64;
    // on those targets a file too large to fit in usize falls through to the
    // chunked path, which is the correct behaviour.
    let filesize = usize::try_from(file.metadata()?.len()).ok();

    if filesize.is_some_and(|size| size <= BUFFER_SIZE) {
        let mut data = Vec::with_capacity(filesize.unwrap_or(0));
        file.read_to_end(&mut data)?;
        return Ok(D::digest(&data));
    }

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
