# HashRust Code Review

**Date:** 2026-02-28
**Reviewer:** Claude Code (claude-sonnet-4-6)
**Commit:** f8e71ad (main)
**Scope:** Full codebase review — current state vs. prior review (2026-02-09)
**Lines reviewed:** ~1,100 Rust

---

## Prior Review Status

The February 9 Opus review identified 22 findings. The current codebase has addressed the majority:

| Prior Finding | Status |
|---|---|
| C1: Silent partial hash on I/O error | **Fixed** — loop now uses `?` |
| C2: TOCTOU race in file_size | **Fixed** — `file_size()` calls `metadata()` directly |
| H1: Interleaved stdout in parallel mode | **Fixed** — results collected then printed serially |
| H2: Loose dependency versions | **Fixed** — standard semver constraints |
| M1: Duplicate CRC32/U32 validation | **Fixed** — validation lives only in CLI layer |
| M2: `OutputEncoding::Unspecified` | **Fixed** — removed; `Option<OutputEncoding>` used during parse |
| M3: Boolean parameter explosion | **Fixed** — struct literal construction, no constructor |
| M4: Integration test temp file races | **Fixed** — `tempfile` crate used throughout |
| M5: No unit tests for core hashing | **Fixed** — `hash_tests` module covers all algorithms |
| M6: No multi-threaded path coverage | **Fixed** — `test_multi_file_parallel_hashing` added |
| L1: Heap buffer per file | **Fixed** — `[u8; BUFFER_SIZE]` stack allocation |
| L2: Unnecessary `.to_string()` | **Fixed** |
| L3: `hash_file_whole` verbosity | **Fixed** — uses `D::digest(&data)` |
| L4: `Crc32::from_state` dead code | **Fixed** — removed |
| L5: Unused `_debug_mode` params | **Fixed** — removed |
| L6: `readonly` setter bypass | **Fixed** — `readonly` removed entirely |
| L7: Redundant `file_exists` | **Fixed** — removed |
| I1: Clippy pedantic violations | **Fixed** |
| I3: Heavyweight progress system | **Fixed** — uses `indicatif::MultiProgress` natively |
| I4: `BasicHash` encapsulation | **Fixed** — inner field is private, public API surface unchanged |

---

## Current Findings

### High

#### H1: Per-file hashing errors do not fail the process
**Files:** `src/core/worker.rs:81`, `src/core/worker.rs:140`

```rust
Err(e) => eprintln!("File error for '{pathstr}': {e}"),
```

Both `file_hashes_st` and `file_hashes_mt` swallow per-file errors: the error is printed to stderr but the functions return `Ok(())`. `worker_func` propagates that `Ok`, so the process exits with code 0.

This is a correctness defect for any scripted consumer. If a caller pipes a list of files and one becomes unreadable mid-run, the process silently succeeds. Standard hashing tools (`sha256sum`, `b2sum`) exit non-zero when any file fails.

**Fix:** Track whether any error occurred (e.g., `let mut had_error = false;`) and return `Err` if set, or use an explicit non-zero exit code via `std::process::exit`.

---

#### H2: Help text printed to stdout on error path
**File:** `src/main.rs:20`

```rust
if let Err(e) = result {
    show_help(true);   // uses println! → stdout
    println!();
    return Err(e);     // anyhow prints this to stderr
}
```

When the CLI returns an error (invalid algorithm, file not found, etc.), help text goes to stdout while the error itself goes to stderr. This mixes streams, breaking piped usage (`./hash_rust bad-arg 2>/dev/null | wc -l` would receive spurious help text) and violating POSIX convention.

**Fix:** Replace `println!` in `show_help` with `eprintln!` when called from the error path, or add a `to_stderr: bool` parameter.

---

### Medium

#### M1: Non-UTF-8 paths from stdin cause a hard failure
**File:** `src/io/files.rs:24`

```rust
let lines = stdin.lock().lines().collect::<Result<Vec<String>, _>>()?;
```

`BufRead::lines()` returns `Err` for any line containing non-UTF-8 bytes. Because the `?` here is applied to the outer `collect`, a single non-UTF-8 filename causes the *entire* stdin read to fail, discarding all subsequent paths.

On Linux, filenames are arbitrary byte sequences. A directory with even one file whose name is not valid UTF-8 will abort stdin mode entirely, even for all the valid paths that preceded it.

**Fix:** Use `stdin.lock().split(b'\n')` which yields raw `Vec<u8>`, then convert with `String::from_utf8_lossy`, or `OsString::from_vec` (Unix-only). At minimum, log-and-skip bad lines rather than aborting.

---

#### M2: Dead `Write` impl on `Crc32`
**File:** `src/hash/crc32.rs:40-51`

```rust
impl Write for Crc32 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { ... }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
```

`Crc32` is used exclusively through the `Digest` trait (`Update` + `FixedOutput`). The `Write` impl is never called anywhere in the codebase. It adds noise, imports `std::io::Write` into scope with a `pub use`, and could mislead a future maintainer into thinking `Crc32` has `Write`-based consumers.

**Fix:** Remove the `impl Write for Crc32` block and the `use std::io::Write` import.

---

#### M3: Integration test path-with-spaces fragility
**File:** `tests/integration_tests.rs:301`

```rust
assert!(
    line.split_whitespace().count() == 2,
    "Line {i} should have exactly 2 space-separated parts (hash and path)"
);
```

The output format is `<hash> <path>`, but this assertion breaks the moment any file path contains whitespace — which is common on macOS where the system temp dir can be `/private/var/folders/...` (though in practice this specific path doesn't contain spaces, user-created temp dirs can). The test will also fail for any future encoding that produces output containing spaces (e.g., Base64 with padding considerations, though that is not currently the case).

**Fix:** Split on the first whitespace only — `line.splitn(2, char::is_whitespace).count() == 2` — or verify the hash prefix length (64 hex chars for SHA3-256) directly.

---

### Low

#### L1: `#[allow(dead_code)]` on public methods is misleading
**File:** `src/core/types.rs:71-80`

```rust
#[allow(dead_code)]
pub fn as_str(&self) -> &str { ... }

#[allow(dead_code)]
pub fn into_inner(self) -> String { ... }
```

`dead_code` lint does not fire on `pub` items in a binary crate — only on private/`pub(crate)` items that are not reachable from the public API. These suppressions have no effect and imply the methods are dead when they are actually part of the intended public interface. They're probably leftovers from when these methods were `pub(crate)`.

**Fix:** Remove the `#[allow(dead_code)]` attributes from both methods.

---

#### L2: `test_empty_supplied_paths` tests nothing
**File:** `src/unit_tests.rs:603-619`

```rust
#[test]
fn test_empty_supplied_paths() {
    let config = ConfigSettings { ..., supplied_paths: Vec::new() };
    assert!(config.supplied_paths.is_empty());
}
```

This constructs a struct and asserts a property of the literal value used to construct it. No code is exercised. The test comment says "This would normally read from stdin, but we're testing the config behavior" — but it doesn't test config behavior either, only struct construction.

**Fix:** Either delete the test, or replace it with an actual behavioral test (e.g., that `get_required_filenames` with empty `supplied_paths` attempts to read stdin and returns an empty vec when stdin is empty).

---

#### L3: `threshold_millis()` wraps a module-level constant
**File:** `src/progress.rs:72-74`

```rust
pub fn threshold_millis() -> u64 {
    PROGRESS_THRESHOLD_MILLIS
}
```

This is a public function that returns a private constant. The only call site is `worker.rs:164`. The indirection adds no encapsulation (the constant's value is fully determined at compile time) and serves no purpose a `pub const` would not serve equally.

**Fix:** Make `PROGRESS_THRESHOLD_MILLIS` `pub(crate)` and reference it directly, or keep the function but note it's an intentional abstraction boundary.

---

#### L4: CLAUDE.md documents `panic = 'abort'` but it is commented out
**File:** `Cargo.toml:6`

```toml
[profile.release]
# panic = 'abort'
```

`CLAUDE.md` states: *"Release builds use LTO and panic=abort for size/speed"*. The setting is commented out. The discrepancy means either the documentation or the config is wrong. Without `panic=abort`, the release binary includes unwinding infrastructure, increasing binary size and disabling the size/speed claim.

**Fix:** Either restore `panic = 'abort'` to the release profile, or remove the claim from CLAUDE.md.

---

#### L5: Commented-out `readonly` dependency is dead config noise
**File:** `Cargo.toml:last dependency block`

```toml
# readonly = "0.2"
```

This was removed as part of fixing L6 from the prior review, but the commented-out line was left behind. It provides no value and could confuse someone scanning the dependency list.

**Fix:** Delete the commented-out line.

---

## Summary

| Severity | Count | Items |
|---|---|---|
| High | 2 | H1 (exit code on errors), H2 (help to stdout) |
| Medium | 3 | M1 (non-UTF-8 stdin), M2 (dead Write impl), M3 (test fragility) |
| Low | 5 | L1–L5 above |
| **Total** | **10** | |

---

## Remediation Sequence

### Immediately (correctness)
1. **H1** — Return non-zero exit code when any file fails to hash
2. **H2** — Route help text to stderr on error path

### Near-term (correctness + test reliability)
3. **M1** — Handle non-UTF-8 stdin paths gracefully
4. **M3** — Fix fragile `split_whitespace` assertion in integration test

### Cleanup (code hygiene)
5. **M2** — Remove `impl Write for Crc32`
6. **L1** — Remove `#[allow(dead_code)]` from public methods
7. **L2** — Delete or replace the no-op `test_empty_supplied_paths`
8. **L4** — Reconcile `panic = 'abort'` between CLAUDE.md and Cargo.toml
9. **L5** — Delete commented-out `readonly` line
10. **L3** — Make `PROGRESS_THRESHOLD_MILLIS` `pub(crate)` and remove the wrapper function
