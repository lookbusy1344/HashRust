use crate::classes::BasicHash;
use byteorder::{BigEndian, ByteOrder};
use digest::{Digest, Output};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 4096;

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
/// Returns Output<D>, which is an owned fixed size array of u8
/// Output<D> = `GenericArray<u8, <D as OutputSizeUser>::OutputSize>`
fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
    if !file_exists(filename) {
        return Err(anyhow::anyhow!("File not found: {}", filename));
    }

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; BUFFER_SIZE];

    let mut hasher = D::new();
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    // Output<T> = GenericArray<u8, <T as OutputSizeUser>::OutputSize>
    // just return this directly to avoid an extra allocation
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
pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    let path_ref = path.as_ref();
    path_ref.exists() && path_ref.is_file()
}

// /// take a string and get the size of the file
// fn file_size(path: &str) -> anyhow::Result<u64> {
//     let path = Path::new(path);
//     if path.exists() && path.is_file() {
//         Ok(path.metadata()?.len())
//     } else {
//         Err(anyhow::anyhow!("File not found: {}", path.display()))
//     }
// }
