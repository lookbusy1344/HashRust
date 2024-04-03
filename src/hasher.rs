use crate::classes::BasicHash;
use byteorder::{BigEndian, ByteOrder};
use digest::{Digest, Output};
use std::fs::File;
use std::io::{BufReader, Read};
//use std::mem::MaybeUninit;
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

#[inline]
pub fn hash_file_hex<D: Digest>(filename: &str) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;
    Ok(BasicHash(hex::encode(h)))
}

#[inline]
pub fn hash_file_u32<D: Digest>(filename: &str) -> anyhow::Result<BasicHash> {
    let h = hash_file::<D>(filename)?;
    let number = BigEndian::read_u32(&h);
    Ok(BasicHash(format!("{number:010}")))
}

// / crc32fast doesnt seem to implement Digest, so we have to have a custom function for it
// pub fn hash_file_crc32(filename: &str) -> anyhow::Result<BasicHash> {
//     if !file_exists(filename) {
//         return Err(anyhow::anyhow!("File not found: {}", filename));
//     }
//
//     let file = File::open(filename)?;
//     let mut reader = BufReader::new(file);
//     let mut buffer = [0u8; BUFFER_SIZE];
//
//     let mut hasher = crc32fast::Hasher::new();
//     loop {
//         let n = reader.read(&mut buffer)?;
//         if n == 0 {
//             break;
//         }
//         hasher.update(&buffer[..n]);
//     }
//
//     // crc32 is just as u32, so we have to convert it to a string
//     let hashnum = hasher.finalize();
//     let result = format!("{:010}", hashnum);
//     Ok(BasicHash(result))
// }

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
