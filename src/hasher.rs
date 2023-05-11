use digest::Digest;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BasicHash(pub String);

impl From<String> for BasicHash {
    fn from(item: String) -> Self {
        BasicHash(item)
    }
}

impl From<&str> for BasicHash {
    fn from(item: &str) -> Self {
        BasicHash(item.to_string())
    }
}

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
pub fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<BasicHash> {
    if !file_exists(filename) {
        return Err(anyhow::anyhow!("File not found: {}", filename));
    }

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; BUFFER_SIZE]; //vec![0; bsize];

    let mut hasher = D::new();
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    // this is now genericarray<u8, size> which implements lowerhex. Basic arrays do not
    let h = hasher.finalize();
    Ok(BasicHash(hex::encode(h)))
}

/// take a string and check if file exists
pub fn file_exists(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.is_file()
}

/// take a string and get the size of the file
#[allow(dead_code)]
fn file_size(path: &str) -> anyhow::Result<u64> {
    let path = Path::new(path);
    if path.exists() && path.is_file() {
        Ok(path.metadata()?.len())
    } else {
        Err(anyhow::anyhow!("File not found: {}", path.display()))
    }
}
