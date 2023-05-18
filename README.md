# HashRust

A toy project to learn Rust

HashRust is a command-line util for hashing files. Supports MD5, SHA1, SHA2 and SHA3.
Multi-threaded by default using Rayon.

Compile using:

```cargo build -r```

It was build on Windows, but has an option to behave in a case-sensitive way for Linux.

Once built, for info try:

```hash_rust -h```

Example usage:

```hash_rust *.txt -a sha3```

The project also includes a Powershell wrapper to parse the output into useful objects.

```
$results = dir *.txt | .\hashfile.ps1 -algorithm sha3
..or..
$results = .\hashfile.ps1 *.txt -algorithm sha3

$results
```
