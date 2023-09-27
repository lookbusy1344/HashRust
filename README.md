# Hash Rust

[![Building HashRust](https://github.com/lookbusy1344/HashRust/actions/workflows/rust.yml/badge.svg)](https://github.com/lookbusy1344/HashRust/actions/workflows/rust.yml)

## A small CLI project to hash files using various algorithms, using Rust

HashRust is a command-line util for hashing files. Supports `MD5, SHA1, SHA2 and SHA3`.
Multi-threaded by default using Rayon. Use a single thread with `-s`


## Building

```cargo build -r```

It was build on Windows, but has an option to behave in a case-sensitive way for Linux.

## Usage

Command line help

```
hash_rust.exe filespec [flags] [options]

Eg:
hash_rust.exe *.txt -a sha2
hash_rust.exe *.txt --algorithm md5 --debug

FLAGS:
    -h, --help                   Prints help information
    -d, --debug                  Debug messages
    -c, --case-sensitive         Case-sensitive glob matching
    -x, --exclude-filenames      Exclude filenames from output
    -s, --single-thread          Single-threaded (not multi-threaded)

OPTIONS:
    -a, --algorithm [algorithm]  Hash algorithm to use
    -l, --limit [num]            Limit number of files processed
    
Algorithm can be:
    MD5, SHA1, 
    SHA2 / SHA2-256, SHA2-384, SHA2-512, 
    SHA3 / SHA3-256 (default), SHA3-384, SHA3-512
```

## Powershell

The project also includes a Powershell wrapper to parse the output into useful objects.

```
$results = dir *.txt | .\hashfile.ps1 -algorithm sha3
..or..
$results = .\hashfile.ps1 *.txt -algorithm sha3

$results
```
