# HashRust

A toy project to learn Rust

HashRust is a command-line util for hashing files. Supports MD5, SHA1, SHA2 and SHA3.
Multi-threaded by default using Rayon.

For info try:

```hash_rust -h```

Example usage:

```hash_rust *.txt -a sha3```

The project also includes a Powershell wrapper to parse the output into useful objects.

```hashfile.ps1 *.txt -algorithm sha3```
