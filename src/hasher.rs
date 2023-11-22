use crate::classes::BasicHash;
use digest::Digest;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 4096;

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
pub fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<BasicHash> {
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

    // this is now genericarray<u8, size> which implements lowerhex. Basic arrays do not
    let hasharray = hasher.finalize();
    Ok(BasicHash(hex::encode(hasharray)))
}

/// crc32fast doesnt seem to implement Digest, so we have to have a custom function for it
pub fn hash_file_crc32(filename: &str) -> anyhow::Result<BasicHash> {
    if !file_exists(filename) {
        return Err(anyhow::anyhow!("File not found: {}", filename));
    }

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; BUFFER_SIZE];

    let mut hasher = crc32fast::Hasher::new();
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    // crc32 is just as u32, so we have to convert it to a string
    let hashnum = hasher.finalize();
    let result = format!("{:010}", hashnum);
    Ok(BasicHash(result))
}

/// take a string and check if file exists
pub fn file_exists(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.is_file()
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
