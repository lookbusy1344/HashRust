use crate::classes::BasicHash;
use byteorder::{BigEndian, ByteOrder};
use digest::{Digest, Output};
use std::fs::File;
use std::io::{BufReader, Read};
use std::mem::MaybeUninit;
use std::path::Path;

const BUFFER_SIZE: usize = 4096 * 8;

/// Hash a file using the given hasher as a Digest implementation, eg `Sha1`, `Sha256`, `Sha3_256`
/// Returns Output<D>, which is an owned fixed size array of u8
/// Output<D> = `GenericArray<u8, <D as OutputSizeUser>::OutputSize>`
fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
    // if !file_exists(filename) {
    //     return Err(anyhow::anyhow!("File not found: {}", filename));
    // }

    let buffersize = usize::try_from(file_size(filename)?)
        .unwrap_or(BUFFER_SIZE)
        .min(BUFFER_SIZE);

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buffer = build_heap_buffer_uninitialized(buffersize);

    // ==== diagnostic code to check if buffer is zeroed
    // count elements in buffer that are not zero
    // let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
    // if non_zero_count != 0 {
    //     eprintln!("Buffer not zeroed {non_zero_count} out of {buffersize} in file {filename}");
    // }
    // ==== end of diagnostic code

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

// /// Hash the entire file at once
// fn hash_file_whole<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
//     // if !file_exists(filename) {
//     //     return Err(anyhow::anyhow!("File not found: {}", filename));
//     // }

//     let data = std::fs::read(filename)?;
//     let mut hasher = D::new();
//     hasher.update(&data);

//     let hasharray = hasher.finalize();
//     Ok(hasharray)
// }

// /// Wrapper function to hash a file. If the file is smaller than `WHOLE_FILE_LIMIT`, it will hash the entire file at once.
// fn hash_file<D: Digest>(filename: &str) -> anyhow::Result<Output<D>> {
//     let size = file_size(filename)?;

//     if size <= WHOLE_FILE_LIMIT {
//         hash_file_whole::<D>(filename)
//     } else {
//         hash_file_buffer::<D>(filename)
//     }
// }

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
#[allow(dead_code)]
fn build_heap_buffer<T: Default + Copy>(len: usize) -> Box<[T]> {
    let vec = vec![T::default(); len];
    vec.into_boxed_slice()
}

/// Build a heap buffer of a given size, uninitialized
fn build_heap_buffer_uninitialized<T: Copy>(len: usize) -> Box<[T]> {
    let vec = vec![MaybeUninit::<T>::uninit(); len];
    let slice = vec.into_boxed_slice();

    convert_maybeunint_to_initialized(slice)
}

/// Convert a `Box<[MaybeUninit<T>]>` to a `Box<[T]>`, unsafe because it doesn't initialize the values
fn convert_maybeunint_to_initialized<T: Copy>(maybe_uninit: Box<[MaybeUninit<T>]>) -> Box<[T]> {
    let len = maybe_uninit.len();
    let raw_ptr = Box::into_raw(maybe_uninit).cast::<T>();

    // now turn it into [T] without initializing the values. This is unsafe because the resulting slice might contain anything
    let result = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(raw_ptr, len)) };
    result
}
