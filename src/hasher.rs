use crate::classes::{BasicHash, OutputEncoding};
use byteorder::{BigEndian, ByteOrder};
use data_encoding::{BASE32, BASE64};
use digest::{Digest, Output};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 4096 * 8;

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
/// Returns Output<D>, which is an owned fixed size array of u8
/// Output<D> = `GenericArray<u8, <D as OutputSizeUser>::OutputSize>`
fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
    let filesize = usize::try_from(file_size(filename)?).ok();

    if filesize.map_or(false, |size| size <= BUFFER_SIZE) {
        // this file is smaller than the buffer size, so we can hash it all at once
        return hash_file_whole::<D>(filename);
    }

    // read the file in chunks
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buffer = build_heap_buffer(BUFFER_SIZE);

    let mut hasher = D::new();
    loop {
        let bytesread = reader.read(&mut buffer)?;
        if bytesread == 0 {
            break; // nothing more to read
        }
        hasher.update(&buffer[..bytesread]);
        if bytesread < BUFFER_SIZE {
            break; // we've reached the end of the file
        }
    }

    // Output<T> = GenericArray<u8, <T as OutputSizeUser>::OutputSize>
    // just return this directly to avoid an extra allocation
    let hasharray = hasher.finalize();
    Ok(hasharray)
}

/// Hash the entire file at once
fn hash_file_whole<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
    let data = std::fs::read(filename)?;
    let mut hasher = D::new();
    hasher.update(&data);

    let hasharray = hasher.finalize();
    Ok(hasharray)
}

/// Hash a file using the given hasher as a Digest implementation, and encode the output
#[inline]
pub fn hash_file_encoded<D: Digest>(
    filename: &str,
    encoding: OutputEncoding,
) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;

    let encoded = match encoding {
        OutputEncoding::Hex => hex::encode(h),
        OutputEncoding::Base64 => BASE64.encode(&h),
        OutputEncoding::Base32 => BASE32.encode(&h),
        OutputEncoding::U32 => {
            // check if h size is 4 bytes
            assert!(h.len() == 4, "Hash size is not 4 bytes, but u32 requested");

            let number = BigEndian::read_u32(&h);
            format!("{number:010}")
        }
        OutputEncoding::Unspecified => {
            return Err(anyhow::anyhow!("Unknown encoding"));
        }
    };

    Ok(BasicHash(encoded))
}

/// check if file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    let path_ref = path.as_ref();
    path_ref.exists() && path_ref.is_file()
}

/// get the size of the file
fn file_size(path: &str) -> anyhow::Result<u64> {
    let path = Path::new(path);
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
