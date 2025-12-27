# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HashRust is a CLI file hashing utility written in Rust that supports multiple hash algorithms (MD5, SHA1, SHA2, SHA3, Blake2, Whirlpool, CRC32) with multi-threading capabilities via Rayon. The project uses a modular architecture with separate modules for hashing logic, algorithm types, and CLI processing.

## Common Commands

### Building
- `cargo build` - Development build
- `cargo build -r` or `cargo build --release` - Release build (recommended for performance)

### Testing
- `cargo test` - Run unit tests (located in src/unit_tests.rs)
- `cargo test --test integration_tests` - Run integration tests

### Code Quality
- `cargo check` - Fast compilation check without producing binary
- `cargo clippy` - Run linter with standard warnings
- `cargo clippy -- -D clippy::all -D clippy::pedantic` - Run pedantic clippy checks (matches .vscode/tasks.json)
- `cargo fmt` - Format code

### Running
- `cargo run -- *.txt -a sha3` - Run with file glob and algorithm
- `cargo run -- --help` - Show help

## Code Architecture

### Module Structure
- `main.rs` - CLI argument parsing, main worker function, and threading coordination
- `core/types.rs` - Core types: `HashAlgorithm` enum, `OutputEncoding` enum, `BasicHash` wrapper
- `cli/config.rs` - Configuration: `ConfigSettings` struct
- `hash/` - Hashing logic and algorithm implementations
- `io/` - File handling and glob pattern matching
- `unit_tests.rs` - Unit test module

### Key Design Patterns
- Uses Rust's `Digest` trait for generic hash algorithm implementation
- Multi-threading via Rayon's parallel iterators (`par_iter`)
- Single-threaded fallback for single files or when explicitly requested
- Enum-driven algorithm selection with `strum` for string parsing
- Buffer size optimization: small files (≤32KB) read entirely, larger files use 32KB chunks

### Algorithm Support
- CRC32 outputs as U32 format only (10-digit zero-padded)
- All other algorithms support Hex (default), Base64, Base32 encoding
- Default algorithm is SHA3-256

### Configuration
- `ConfigSettings` struct centralizes all CLI options
- Supports glob patterns with case-sensitive option
- File input via CLI args or stdin pipe
- Optional file count limiting

### Error Handling
- Uses `anyhow` for error propagation
- Graceful handling of file access errors
- Validates algorithm/encoding combinations

## Development Notes

### Copilot Instructions Integration
- Target senior engineers (15+ years experience)
- Use modern Rust idioms and functional style
- Keep code concise, avoid unchanged code in suggestions
- Brief commit messages preferred

### Performance Considerations
- Multi-threading is default behavior
- Buffer size optimized for typical file sizes
- Direct file I/O with large buffers for performance
- Release builds use LTO and panic=abort for size/speed

## Development Workflow

**IMPORTANT: After making code changes:**
1. Always run `cargo fmt` to ensure consistent formatting
2. Run `cargo clippy --color=always -- -D clippy::all -D clippy::pedantic` to catch potential issues and ensure code quality

**Security:**
- Run `cargo audit` once a day when working on this project to check for security vulnerabilities in dependencies