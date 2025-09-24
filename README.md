# Hash Rust

[![Build and test](https://github.com/lookbusy1344/HashRust/actions/workflows/rust.yml/badge.svg)](https://github.com/lookbusy1344/HashRust/actions/workflows/rust.yml)

## A small CLI project to hash files using various algorithms, using Rust

HashRust is a command-line util for hashing files. Supports `MD5, SHA1, SHA2, SHA3, Blake2` and `Whirlpool`.
Multi-threaded by default using Rayon.

## Building

```cargo build -r```

It was build on Windows, but has an option to behave in a case-sensitive way for Linux.

## Usage

```
hash_rust filespec [flags] [options]

For example:
  hash_rust *.txt -a sha2
  hash_rust *.txt --algorithm md5 --debug

Or pipe in a list of files:
  dir *.txt /b | hash_rust
```

## Flags

```
    -h, --help                   Prints help information
    -d, --debug                  Debug messages
    -c, --case-sensitive         Case-sensitive glob matching
    -x, --exclude-filenames      Exclude filenames from output
    -s, --single-thread          Single-threaded (not multi-threaded)
```

## Options

```
    -a, --algorithm [algorithm]  Hash algorithm to use
    -e, --encoding [encoding]    Encoding to use (hex, base64, base32)
    -l, --limit [num]            Limit number of files processed (eg only process the first one)
```

CRC32 can only be output as 32-bit integer, the `-e` option cannot be used with it.

## Algorithms supported

```
    MD5, SHA1,
    WHIRLPOOL, BLAKE2S-256, BLAKE2B-512,
    SHA2 / SHA2-256, SHA2-224, SHA2-384, SHA2-512, 
    SHA3 / SHA3-256, SHA3-384, SHA3-512

    The default is SHA3-256
```

## Powershell integration

The project also includes a Powershell wrapper to parse the output into useful objects.

```
$results = dir *.txt | .\hashfile.ps1 -algorithm sha3
..or..
$results = .\hashfile.ps1 *.txt -algorithm sha3

$results
```
