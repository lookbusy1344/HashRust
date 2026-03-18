# HashRust Code Review — Remediation Plan

**Date:** 2026-03-18
**Reviewer:** Claude (automated full-codebase audit)
**Test status at review time:** 49 tests passing, pedantic clippy clean

---

## Executive Summary

No critical issues found. The safety story is strong (`#![forbid(unsafe_code)]`, `anyhow` throughout, no panicking code in production paths). Three high-severity issues warrant attention before the next release, eight medium issues should be addressed in normal development flow, and six low issues are housekeeping items.

---

## High Severity

### H1 — SIGPIPE / Broken Pipe Aborts Process in Release Builds

**File:** `src/core/worker.rs`, lines 82–84, 147–149
**Impact:** `hash_rust *.dat | head -5` kills the process with a non-zero exit and no diagnostic in release builds (where `panic = 'abort'` is set).

`println!` flushes through the global stdout lock. On Unix, if the downstream consumer closes the pipe, the write fails with `BrokenPipe`. In debug builds this panics; in release builds with `panic = 'abort'` it aborts silently with a non-zero exit. There is no SIGPIPE handling and no `BufWriter` wrapping stdout.

**Remediation options (pick one):**

Option A — Signal reset (requires `libc` dependency):
```rust
// at top of main(), before any I/O
#[cfg(unix)]
unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL); }
```

Option B — Wrap stdout in `BufWriter`, propagate `flush()` error at exit from `worker_func`:
```rust
let stdout = std::io::BufWriter::new(std::io::stdout());
// write all output through this, then flush() and propagate the Result
```

**Suggested fix:** Option B avoids an `unsafe` block and a new dependency. Refactor `worker_func` to accept or construct a `BufWriter<Stdout>` and return its flush error as part of the `Result<()>`.

---

### H2 — Silent `.ok()` on `try_from` Obscures Intent

**File:** `src/hash/digest_impl.rs`, line 14
**Impact:** Low on 64-bit; a future port to 32-bit would silently downgrade all files > 2 GB to chunked path without any warning.

```rust
let filesize = usize::try_from(file_size(filename.as_ref())?).ok();
```

The `.ok()` converts an arithmetic overflow on `try_from` to `None`, silently falling through to the chunked path. The intent is correct but undocumented — easy to misread as "both errors are being swallowed".

**Remediation:**
```rust
// usize::try_from can only fail on 32-bit targets where usize < u64;
// falling back to None (chunked path) is the correct behaviour there.
let filesize = usize::try_from(file_size(filename.as_ref())?).ok();
```
Add the comment above. Optionally restructure to an explicit match for clarity.

---

### H3 — Six Hash Algorithms Have No Known-Answer Tests

**File:** `src/unit_tests.rs`
**Impact:** A mis-wired dispatch in `call_hasher` or a silent dependency change could produce wrong hashes with no test failure.

The following variants have no known-answer test (KAT):

| Variant | Expected hex length |
|---|---|
| `SHA2_224` | 56 |
| `SHA2_384` | 96 |
| `SHA2_512` | 128 |
| `SHA3_384` | 96 |
| `SHA3_512` | 128 |
| `Blake2B512` | 128 |

`Whirlpool` and `Blake2S256` have tests but only validate structural properties (length, hex charset), not digest values.

**Remediation:** Add one KAT per untested variant using a precomputed reference (Python `hashlib`, OpenSSL `dgst`, or NIST test vectors). Promote existing structural tests for `Whirlpool` and `Blake2S256` to value tests.

Example pattern:
```rust
#[test]
fn test_sha2_224_known_answer() {
    let result = call_hasher(HashAlgorithm::SHA2_224, OutputEncoding::Hex, b"abc");
    assert_eq!(result.unwrap().as_str(), "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7");
}
```

---

## Medium Severity

### M1 — Magic Numbers in `progress.rs` Violate CLAUDE.md

**File:** `src/progress.rs`, lines 32 and 64
**CLAUDE.md:** "Avoid magic numbers. Use named constants or enumerations instead."

```rust
if file_count < 10 {   // threshold for overall bar vs. spinners
pb.enable_steady_tick(Duration::from_millis(350));
```

**Remediation:**
```rust
const OVERALL_BAR_FILE_THRESHOLD: usize = 10;
const SPINNER_TICK_MS: u64 = 350;
```

---

### M2 — Unnecessary `Send` Bound on Single-Threaded Function

**File:** `src/core/worker.rs`, line 60
**Impact:** Cosmetic — restricts callers unnecessarily, misleads readers about concurrency requirements.

`file_hashes_st` runs on a single thread. The `Send + Sync` bounds were copied from the MT variant. `par_iter` over `&[S]` in `file_hashes_mt` only requires `S: Sync`, not `S: Send`.

**Remediation:** Remove `Send` from `file_hashes_st`. Verify and remove `Send` from `file_hashes_mt` as well.

---

### M3 — `Arc<ProgressBar>` is Redundant

**File:** `src/progress.rs`, line 46; `src/core/worker.rs`, lines 112–137
**Impact:** One extra allocation and pointer indirection per progress bar for no benefit.

`ProgressBar` from `indicatif` is already internally `Arc`-backed and is `Clone + Send + Sync`. The extra `Arc<ProgressBar>` wrapper adds nothing.

**Remediation:** Return `Option<ProgressBar>` from `create_overall_progress`. The `par_iter` closure can capture `&overall_progress` by reference.

---

### M4 — Over-Broad `pub` Visibility on Internal Hash Modules

**File:** `src/hash/mod.rs`, lines 1–3

```rust
pub mod algorithms;
pub mod crc32;
pub mod digest_impl;
```

`crc32` and `digest_impl` are implementation details. They are not part of any public API (this is a binary crate).

**Remediation:** Change to `pub(crate)`:
```rust
pub mod algorithms;
pub(crate) mod crc32;
pub(crate) mod digest_impl;
```

---

### M5 — `process::exit(1)` Bypasses All Drop Handlers

**File:** `src/core/worker.rs`, line 38
**Impact:** `indicatif` `MultiProgress` is not cleaned up; `Drop` implementations are skipped. Architecturally inconsistent with the `-> Result<()>` return type.

```rust
if had_error {
    std::process::exit(1);
}
```

**Remediation:** Return `Err(anyhow!("One or more files failed to hash"))` and let `main` call `std::process::exit(1)` after printing the error. This restores consistency with the `Result` return type and allows `Drop` to run.

---

### M6 — Glob Case-Sensitivity Test is Non-Deterministic on macOS

**File:** `src/unit_tests.rs`, line 367
**Impact:** The test passes on case-insensitive APFS/HFS+ volumes (macOS default) even when the glob logic is broken for case-sensitive matching.

The test creates `test1.txt`, `test2.txt`, `Test3.TXT` and expects exactly 2 matches with `case_sensitive: true`. On macOS the OS returns all three from the directory listing; the glob library's case-sensitivity setting only controls pattern character comparison, not filesystem enumeration.

**Remediation:** Either gate the test to Linux-only:
```rust
#[cfg(target_os = "linux")]
#[test]
fn test_glob_wildcard_case_sensitive() { ... }
```
Or restructure to use patterns that are distinguished at the pattern level (e.g., `*.txt` vs `*.TXT` as separate patterns) rather than relying on OS-level case filtering.

---

### M7 — No Test Covers the Overall Progress Bar Path (≥10 Files)

**File:** `tests/integration_tests.rs`, line 273
**Impact:** Bugs in `create_overall_progress`'s `ProgressBar::new` branch go undetected.

The existing parallel test creates 5 files, below the 10-file threshold. The `ProgressBar::new` path in `progress.rs` is never exercised.

**Remediation:** Add an integration test (or a unit test of `ProgressCoordinator`) that hashes 10+ files and verifies the correct number of output lines.

---

### M8 — Stdin I/O Errors Silently Dropped in Non-Debug Mode

**File:** `src/io/files.rs`, lines 35–39

```rust
Err(e) => {
    if config.debug_mode {
        eprintln!("Skipping unreadable stdin line: {e}");
    }
    None
}
```

A genuine stdin read error is swallowed unless `--debug` is active. The user sees a successful exit with fewer files processed than expected.

**Remediation:** Always emit the error to stderr, matching the behaviour of individual file errors in `worker.rs`:
```rust
Err(e) => {
    eprintln!("Error reading stdin: {e}");
    None
}
```
Optionally treat this as a fatal error rather than a skip.

---

## Low Severity

### L1 — Misleading Test Comment re: CRC32 Encoding

**File:** `tests/integration_tests.rs`, line 190

```rust
// CRC32 should only work with Hex encoding (U32 format)
```

CRC32 does **not** work with Hex encoding — it uses U32 format only. The comment contradicts itself.

**Remediation:** Correct to: `// CRC32 only supports U32 encoding (10-digit zero-padded)`

---

### L2 — `pub` + `#[allow(dead_code)]` on Test-Only Methods

**File:** `src/core/types.rs`, lines 71, 78

`BasicHash::as_str()` and `BasicHash::into_inner()` are `pub` but only used in `#[cfg(test)]` code.

**Remediation:** Gate them to test configuration:
```rust
#[cfg(test)]
pub fn as_str(&self) -> &str { ... }
#[cfg(test)]
pub fn into_inner(self) -> String { ... }
```
This removes the need for `#[allow(dead_code)]`.

---

### L3 — Whirlpool and Large-File Tests Don't Assert Digest Values

**File:** `src/unit_tests.rs`, lines 265–278, 306–317

Both `test_call_hasher_large_file` and `test_call_hasher_whirlpool` check only structural properties. See H3 for the fuller picture.

**Remediation:** Add value assertions using precomputed reference digests.

---

### L4 — Trailing Whitespace in `HELP` Constant

**File:** `src/cli/config.rs`, line 31

Line 31 is a blank line inside the `HELP` string literal that contains trailing spaces. Causes diff noise.

**Remediation:** Remove the trailing spaces from the blank line. `cargo fmt` will not catch this (it's inside a string literal).

---

### L5 — `test_config_settings_new` Doesn't Assert All Fields

**File:** `src/unit_tests.rs`, lines 76–96

`single_thread` and `no_progress` are set in the constructed `ConfigSettings` but never asserted.

**Remediation:** Add assertions for all fields set in the test fixture.

---

### L6 — No Per-Test Execution Timeout for Spawned Processes

**File:** `tests/integration_tests.rs`

Each integration test spawns `cargo run`. A hung child (e.g., stdin test blocking forever) would hang the entire test suite indefinitely. CLAUDE.md recommends `gtimeout` at the runner level, but no in-process timeout exists.

**Remediation:** Add `wait-timeout` crate and wrap `child.wait()` calls, or use `child.wait_timeout(Duration::from_secs(30))` to enforce a per-test deadline.

---

## Remediation Priority

| Priority | Issues | Rationale |
|---|---|---|
| **Now** (before next release) | H1, H3 | User-visible breakage (SIGPIPE), correctness confidence |
| **Next sprint** | H2, M1, M5, M6, M8 | Code clarity, CLAUDE.md compliance, test reliability |
| **Backlog** | M2, M3, M4, M7, L1–L6 | Architectural tidiness, coverage gaps, housekeeping |

---

## Checklist

- [ ] H1: Handle SIGPIPE / wrap stdout in BufWriter
- [ ] H2: Add explanatory comment to `.ok()` on `try_from`
- [ ] H3: Add known-answer tests for SHA2_224, SHA2_384, SHA2_512, SHA3_384, SHA3_512, Blake2B512
- [ ] M1: Extract `OVERALL_BAR_FILE_THRESHOLD` and `SPINNER_TICK_MS` constants
- [ ] M2: Remove `Send` bound from `file_hashes_st` (and verify `file_hashes_mt`)
- [ ] M3: Remove `Arc` wrapper from `Option<Arc<ProgressBar>>`
- [ ] M4: Change `crc32` and `digest_impl` to `pub(crate)`
- [ ] M5: Replace `process::exit(1)` with `Err(...)` return, handle in `main`
- [ ] M6: Gate case-sensitivity glob test to Linux or restructure
- [ ] M7: Add integration test for ≥10 file path (overall progress bar)
- [ ] M8: Always emit stdin I/O errors to stderr
- [ ] L1: Fix misleading CRC32 test comment
- [ ] L2: Gate `BasicHash::as_str` and `into_inner` to `#[cfg(test)]`
- [ ] L3: Add value assertions to Whirlpool and large-file tests
- [ ] L4: Remove trailing whitespace from HELP constant blank line
- [ ] L5: Assert all fields in `test_config_settings_new`
- [ ] L6: Add per-test timeout for integration test child processes
