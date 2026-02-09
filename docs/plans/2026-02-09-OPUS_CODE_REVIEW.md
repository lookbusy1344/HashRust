# HashRust Code Review & Staged Remediation Plan

**Date:** 2026-02-09
**Reviewer:** Claude Code
**Commit:** 240a599 (main)
**Scope:** Full codebase review (~1,025 lines Rust)

---

## Executive Summary

HashRust is a well-structured CLI hashing utility. The module separation is clean, the algorithm abstraction via the `Digest` trait is sound, and the CRC32 adapter is a clever use of Rust's trait system. That said, there are a handful of correctness issues (one of which can silently produce wrong hashes), a dependency versioning strategy that will eventually break the build, and meaningful test coverage gaps in the core hashing path.

22 findings are categorised below by severity, then grouped into a staged remediation plan.

---

## Findings

### Critical (Correctness)

#### C1: Silent partial hashes on I/O error
**File:** `src/hash/digest_impl.rs:24-29`

```rust
while let Ok(bytes_read) = file.read(&mut buffer) {
    if bytes_read == 0 { break; }
    hasher.update(&buffer[..bytes_read]);
}
```

If `file.read()` returns `Err` mid-file (bad sector, NFS timeout, permission change), the loop breaks silently and `finalize()` returns a hash of the *partial* data. For a hashing tool, this is a data-integrity defect — the caller has no indication the hash is wrong.

**Fix:** Propagate the error with `?` instead of breaking on `Err`.

---

#### C2: TOCTOU race in file_size / file_exists
**Files:** `src/hash/digest_impl.rs:66-73`, `src/io/files.rs:8-11`

`file_size()` checks `path.exists() && path.is_file()` before returning metadata. Between this check and the subsequent `File::open()`, the file can be deleted or replaced. `file_exists()` has the same pattern.

**Fix:** Remove the pre-check in `file_size()`. Just call `std::fs::metadata()` and let it fail. In `hash_file()`, attempt to open the file directly and handle the error — the OS open call is the authoritative check.

---

### High

#### H1: Interleaved stdout in parallel mode
**File:** `src/core/worker.rs:94-116`

`println!` inside `par_iter().for_each()` is not atomic. Two threads writing simultaneously can produce garbled lines (e.g., a hash from one file on the same line as a filename from another). This is especially likely when progress bars are also writing to stderr.

**Fix:** Collect results into a `Vec` from `par_iter`, then print sequentially. Alternatively, lock stdout once per write: `writeln!(io::stdout().lock(), ...)`.

---

#### H2: Loose dependency version constraints
**File:** `Cargo.toml:15-34`

Every dependency uses `>=` (e.g., `>= 1.0.98`). This permits any future major version. A breaking release of *any* dependency will break the build on a fresh `cargo update`. The `digest` constraint `> 0.10.5` is even more permissive and would accept a hypothetical `0.11` or `1.0` with an entirely different API.

**Fix:** Use default semver-compatible constraints (`"1.0.98"` which means `^1.0.98`). Pin the `digest` ecosystem crates to `"0.10.x"` ranges since they have a tightly-coupled API.

---

### Medium

#### M1: Duplicated CRC32/U32 validation
**Files:** `src/cli/args.rs:28-34`, `src/hash/algorithms.rs:18-24`

The same validation (CRC32 ↔ U32 coupling) is implemented in two places. If one is updated, the other can silently diverge.

**Fix:** Validate in one place only (CLI layer). The hasher can trust its inputs have been validated. If defence-in-depth is desired, use `debug_assert!` in the hasher instead.

---

#### M2: `OutputEncoding::Unspecified` leaks into the type system
**File:** `src/core/types.rs:54`

`Unspecified` is an internal sentinel used during CLI parsing to mean "user didn't pass `-e`". It then gets resolved to `Hex` or `U32` in `args.rs:22-26`. But it remains a variant of the public enum, so every `match` on `OutputEncoding` must handle it. In `digest_impl.rs:50` it's treated as `Hex`, creating a silent fallback.

**Fix:** Use `Option<OutputEncoding>` during parsing and resolve to a concrete encoding before constructing `ConfigSettings`. Remove the `Unspecified` variant.

---

#### M3: ConfigSettings constructor — boolean parameter explosion
**File:** `src/cli/config.rs:21-42`

Five positional booleans with `#[allow(clippy::fn_params_excessive_bools)]` and `#[allow(clippy::too_many_arguments)]`. Callers must remember the exact parameter order; swapping two bools compiles silently.

**Fix:** Use a builder pattern, or construct the struct directly with named fields (Rust struct literal syntax already prevents ordering mistakes if the constructor is removed).

---

#### M4: Integration tests race on shared temp files
**File:** `tests/integration_tests.rs`

Tests use fixed filenames like `test_file.txt`, `test_md5.txt` in the system temp directory. `cargo test` runs tests in parallel by default, and the cleanup (`fs::remove_file`) from one test can race with another that's still running.

**Fix:** Use the `tempfile` crate or `std::env::temp_dir().join(format!("hashrust_test_{}", uuid))` to generate unique paths.

---

#### M5: No unit tests for core hashing logic
**Files:** `src/hash/digest_impl.rs`, `src/hash/algorithms.rs`

`hash_file`, `hash_file_encoded`, and `call_hasher` have zero unit test coverage. All verification is through integration tests that shell out to `cargo run`, making failures slow to diagnose and impossible to isolate to the hashing layer.

**Fix:** Add unit tests that hash known byte sequences and assert against precomputed digests. Test each algorithm and encoding combination.

---

#### M6: No test coverage for multi-threaded path
**File:** `src/core/worker.rs:79-121`

Every integration test hashes a single file, so `file_hashes_mt` is never exercised. Any concurrency bugs (like H1) would not be caught.

**Fix:** Add integration tests that hash multiple files simultaneously.

---

### Low

#### L1: Heap buffer allocation per file
**File:** `src/hash/digest_impl.rs:21`

`build_heap_buffer(BUFFER_SIZE)` allocates a fresh 32KB `Box<[u8]>` for every file. In the multi-threaded path with many small files, this creates allocation pressure.

**Fix:** Use a stack-allocated `[u8; BUFFER_SIZE]` (32KB is well within typical stack limits), or use a thread-local buffer.

---

#### L2: Unnecessary `.to_string()` in hot path
**Files:** `src/core/worker.rs:64,97`

`pathstr.as_ref().to_string()` allocates a new `String` just to pass it to `call_hasher`, which only needs `AsRef<str>`.

**Fix:** Pass `pathstr.as_ref()` directly since `call_hasher` already accepts `impl AsRef<str>`.

---

#### L3: `hash_file_whole` doesn't use `Digest::digest` one-liner
**File:** `src/hash/digest_impl.rs:34-40`

The `Digest` trait provides `D::digest(&data)` which is equivalent to `new() → update() → finalize()`. The current code manually does all three steps.

**Fix:** Replace with `Ok(D::digest(&data))`.

---

#### L4: `#[allow(dead_code)]` on `Crc32::from_state`
**File:** `src/hash/crc32.rs:11,19-21`

`from_state` is unused and marked `#[allow(dead_code)]`. Dead code should either be used or removed.

**Fix:** Remove `from_state` and the `#[allow(dead_code)]` attribute.

---

#### L5: Unused `_debug_mode` parameters
**File:** `src/progress.rs:51,100`

`create_file_progress` and `create_overall_progress` accept `_debug_mode` but never use it. This is dead interface surface area.

**Fix:** Remove the parameters if there's no planned use. If debug logging is intended, implement it or add a `// TODO` with a tracking issue.

---

#### L6: `readonly` crate bypassed by `set_supplied_paths`
**File:** `src/cli/config.rs:4,44-46`

`#[readonly::make]` generates compile errors for direct field mutation, but `set_supplied_paths` provides a public setter that mutates `supplied_paths` anyway. This makes `readonly` provide a false sense of safety.

**Fix:** Build the struct fully (including paths) in `process_command_line` and remove the setter. Then `readonly` actually means something, or remove the `readonly` dependency entirely and rely on module visibility.

---

#### L7: `file_exists` is redundant
**File:** `src/io/files.rs:8-11`

`path.exists() && path.is_file()` can be replaced with `path.is_file()` (which returns `false` for nonexistent paths). But more importantly, this function exists only to be called before an operation that will fail with a clear OS error anyway (see C2).

**Fix:** Remove `file_exists` where it guards file operations. Keep it only where the existence check is the goal (e.g., filtering stdin lines).

---

### Informational / Style

#### I1: Clippy pedantic violations
**File:** `src/io/files.rs:74,86-88`

Two `uninlined_format_args` violations caught by `cargo clippy -- -D clippy::pedantic`. These are currently blocking a clean clippy run.

---

#### I2: No `cargo audit` issues
All 59 crate dependencies passed with no known vulnerabilities as of 2026-02-09.

---

#### I3: Progress system is heavyweight
**File:** `src/progress.rs`

Spawning OS threads with channels for spinner display when `indicatif` already provides `MultiProgress` for coordinated progress bars. The global `AtomicUsize` counter and `MAX_PROGRESS_THREADS` limit suggest this was built to work around concurrency issues that `MultiProgress` solves natively.

---

#### I4: `BasicHash` provides no encapsulation
**File:** `src/core/types.rs:57-64`

`BasicHash(pub String)` — the inner field is public, so the newtype provides zero invariant enforcement. It's a type alias with extra syntax.

---

---

## Staged Remediation Plan

### Stage 1: Correctness (Do First)

These items fix bugs that produce wrong results or undefined behaviour.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 1 | Fix silent partial hash on I/O error (C1) | `digest_impl.rs` | Small |
| 2 | Remove TOCTOU pre-checks (C2) | `digest_impl.rs`, `files.rs` | Small |
| 3 | Fix interleaved stdout in parallel mode (H1) | `worker.rs` | Medium |

**Verification:** Run existing tests. Hash a large file and compare against `sha3sum`. Manually test with 100+ files to check output interleaving.

---

### Stage 2: Dependency Hygiene

These prevent future build breakage.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 4 | Pin dependency versions to semver-compatible ranges (H2) | `Cargo.toml` | Small |
| 5 | Fix clippy pedantic violations (I1) | `files.rs` | Trivial |

**Verification:** `cargo clippy -- -D clippy::all -D clippy::pedantic` passes clean. `cargo update` still resolves.

---

### Stage 3: Test Coverage

Fill the most impactful test gaps.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 6 | Add unit tests for `hash_file_encoded` / `call_hasher` (M5) | New test module or `unit_tests.rs` | Medium |
| 7 | Add multi-file integration test (M6) | `integration_tests.rs` | Small |
| 8 | Fix temp file race in integration tests (M4) | `integration_tests.rs` | Small |
| 9 | Add stdin path tests | `integration_tests.rs` | Small |
| 10 | Add glob edge-case tests | `unit_tests.rs` or `integration_tests.rs` | Medium |

**Verification:** `cargo test` passes. Coverage increases for `hash/` and `io/` modules.

---

### Stage 4: API / Type Cleanup

Improve internal API correctness and remove dead code.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 11 | Remove `OutputEncoding::Unspecified` — use `Option` during parsing (M2) | `types.rs`, `args.rs`, `config.rs`, `digest_impl.rs` | Medium |
| 12 | Deduplicate CRC32/U32 validation (M1) | `args.rs`, `algorithms.rs` | Small |
| 13 | Remove `Crc32::from_state` dead code (L4) | `crc32.rs` | Trivial |
| 14 | Remove unused `_debug_mode` params or implement debug logging (L5) | `progress.rs` | Trivial |
| 15 | Remove `file_exists` where redundant (L7) | `files.rs` | Small |
| 16 | Build `ConfigSettings` fully before returning, remove setter and `readonly` workaround (L6) | `config.rs`, `args.rs` | Small |

**Verification:** `cargo clippy -- -D clippy::all -D clippy::pedantic` clean. All tests pass.

---

### Stage 5: Constructor & Ergonomics (Optional)

Lower priority improvements to code ergonomics.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 17 | Replace boolean-heavy constructor with builder or struct literal (M3) | `config.rs`, `args.rs`, `unit_tests.rs` | Medium |
| 18 | Simplify `hash_file_whole` with `D::digest` (L3) | `digest_impl.rs` | Trivial |
| 19 | Replace heap buffer with stack allocation (L1) | `digest_impl.rs` | Trivial |
| 20 | Remove unnecessary `.to_string()` allocations (L2) | `worker.rs` | Small |

**Verification:** Benchmark before/after with `hyperfine` on a directory of mixed-size files.

---

### Stage 6: Architectural (Future / Optional)

Larger refactors with lower urgency.

| # | Finding | Files | Effort |
|---|---------|-------|--------|
| 21 | Replace custom progress threading with `indicatif::MultiProgress` (I3) | `progress.rs`, `worker.rs` | Large |
| 22 | Give `BasicHash` real invariants or replace with `String` (I4) | `types.rs`, all consumers | Medium |

**Verification:** Full test suite. Manual UX testing of progress bars with various file counts.

---

## Summary

| Severity | Count |
|----------|-------|
| Critical | 2 |
| High | 2 |
| Medium | 6 |
| Low | 7 |
| Informational | 4 |
| **Total** | **22** |

Stages 1-2 should be addressed promptly — they fix correctness bugs and a ticking build-stability issue. Stage 3 provides the safety net for everything that follows. Stages 4-6 can be prioritised at your discretion.
