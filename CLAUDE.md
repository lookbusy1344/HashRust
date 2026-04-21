# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HashRust is a CLI file hashing utility written in Rust that supports multiple hash algorithms (MD5, SHA1, SHA2, SHA3, Blake2, Whirlpool, CRC32) with multi-threading via Rayon. Modular architecture with separate modules for hashing logic, algorithm types, and CLI processing.

## Common Commands

### Building
- `cargo build` - Development build
- `cargo build --release` - Release build (LTO + panic=abort; recommended for performance)

### Testing
- `cargo nextest run` - Run all tests (preferred)
- `cargo nextest run --test integration_tests` - Integration tests only
- `cargo test` - Fallback if nextest unavailable

### Code Quality
- `cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code`
- `cargo fmt`

### Running
- `cargo run -- *.txt -a sha3` - Run with file glob and algorithm
- `cargo run -- --help`

## Code Architecture

### Module Structure
- `main.rs` - CLI argument parsing, main worker function, threading coordination
- `core/types.rs` - `HashAlgorithm` enum, `OutputEncoding` enum, `BasicHash` wrapper
- `cli/config.rs` - `ConfigSettings` struct
- `hash/` - Hashing logic and algorithm implementations
- `io/` - File handling and glob pattern matching
- `unit_tests.rs` - Unit tests

### Key Design Patterns
- `Digest` trait for generic hash algorithm implementation
- Multi-threading via Rayon `par_iter`; single-threaded fallback for single files or when requested
- Enum-driven algorithm selection with `strum` for string parsing
- Buffer size: small files (≤32KB) read entirely, larger files use 32KB chunks
- Minimise cloning

### Algorithm Support
- CRC32: U32 format only (10-digit zero-padded)
- All other algorithms: Hex (default), Base64, Base32 encoding
- Default algorithm: SHA3-256

### Configuration
- `ConfigSettings` centralises all CLI options
- Glob patterns with case-sensitive option
- File input via CLI args or stdin pipe
- Optional file count limiting

### Error Handling
- `anyhow` for error propagation
- Graceful handling of file access errors
- Validates algorithm/encoding combinations

## Development Workflow

Use the `pre-commit` skill before committing: runs `cargo fmt` and pedantic `cargo clippy`.
