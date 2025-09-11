use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use byteorder::{BigEndian, ByteOrder};
use data_encoding::{BASE32, BASE64};
use digest::{Digest, Output};

use crate::classes::{BasicHash, OutputEncoding};

/// Buffer size for reading large files in chunks. 32KB provides optimal performance
/// by balancing memory usage with I/O efficiency. Larger buffers reduce syscall overhead
/// but increase memory pressure, while smaller buffers result in more frequent I/O operations.
/// Files ≤32KB are read entirely into memory for maximum performance.
const BUFFER_SIZE: usize = 4096 * 8;

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
/// Returns Output<D>, which is an owned fixed size array of u8
/// Output<D> = `GenericArray<u8, <D as OutputSizeUser>::OutputSize>`
fn hash_file<D: Digest>(filename: impl AsRef<str>) -> anyhow::Result<Output<D>> {
    let filesize = usize::try_from(file_size(filename.as_ref())?).ok();

    if filesize.is_some_and(|size| size <= BUFFER_SIZE) {
        // Small file optimization - hash it all at once
        return hash_file_whole::<D>(filename);
    }

    // For larger files, read in chunks
    let file = File::open(filename.as_ref())?;
    let mut reader = BufReader::new(file);
    let mut buffer = build_heap_buffer(BUFFER_SIZE);
    let mut hasher = D::new();

    // More efficient reading pattern
    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

/// Hash the entire file at once
fn hash_file_whole<D: Digest>(filename: impl AsRef<str>) -> anyhow::Result<Output<D>> {
    let data = std::fs::read(filename.as_ref())?;
    let mut hasher = D::new();
    hasher.update(&data);

    Ok(hasher.finalize())
}

/// Hash a file using the given hasher as a Digest implementation, and encode the output
#[inline]
pub fn hash_file_encoded<D: Digest>(
    filename: impl AsRef<str>,
    encoding: OutputEncoding,
) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;

    // Convert hash directly to BasicHash without separate variable
    Ok(BasicHash(match encoding {
        OutputEncoding::Hex | OutputEncoding::Unspecified => hex::encode(h),
        OutputEncoding::Base64 => BASE64.encode(&h),
        OutputEncoding::Base32 => BASE32.encode(&h),
        OutputEncoding::U32 => {
            // check if h size is 4 bytes
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

/// check if file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    let path_ref = path.as_ref();
    path_ref.exists() && path_ref.is_file()
}

/// get the size of the file
fn file_size(path: impl AsRef<str>) -> anyhow::Result<u64> {
    let path = Path::new(path.as_ref());
    if path.exists() && path.is_file() {
        Ok(path.metadata()?.len())
    } else {
        Err(anyhow::anyhow!("File not found: {}", path.display()))
    }
}

/// Build a heap buffer of a given size, filled with default values
fn build_heap_buffer<T: Default + Copy>(len: usize) -> Box<[T]> {
    let vec = vec![T::default(); len];
    vec.into_boxed_slice()
}
