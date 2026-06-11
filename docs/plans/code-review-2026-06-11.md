# HashRust Code Review — Remediation Plan

**Date:** 2026-06-11
**Reviewer:** Claude (fresh-eyes full-codebase audit)
**Baseline at review time:** `cargo build` clean · pedantic clippy clean (`-D clippy::all -D clippy::pedantic -F unsafe_code`) · 59/59 tests passing · `cargo audit` reports no advisories

---

## Executive Summary

No critical or high-severity issues. The prior remediation plan (`code-review-remediation-2026-03-18.md`) has been fully implemented: stdout is `BufWriter`-wrapped with broken-pipe handling, `process::exit` is replaced by a typed `FileHashError`, all algorithms have known-answer tests, magic numbers are named constants, and internal modules are `pub(crate)`.

This pass found one behavioural wart (duplicate hashing of overlapping inputs), one robustness/efficiency item (double `stat`/`open` per file), and one dependency-hygiene item (a pre-release dependency shipping in a tagged release). None block a release; all are cheap.

---

## Medium Severity

### M1 — Overlapping globs / repeated arguments hash the same file twice

**File:** `src/io/files.rs`, `get_paths_matching_glob` (lines 52–92)
**Symptom:** `hash_rust *.txt notes.txt` (where `notes.txt` matches `*.txt`), or `hash_rust a.txt a.txt`, emits two identical output lines and hashes the file twice. With `--limit N`, duplicates also consume slots, so fewer distinct files are processed than the user expects.

`result.extend(glob_matches)` accumulates per-pattern with no de-duplication across patterns or against literal pushes.

**Decision required:** Is de-duplication desired? For a hashing tool it almost always is, but confirm before changing observable output.

**Remediation (if yes):** De-duplicate while preserving first-seen order, after all patterns are collected and **before** `limit` is applied in `get_required_filenames`:

```rust
// in get_required_filenames, after building `paths`, before truncate:
let mut seen = std::collections::HashSet::new();
paths.retain(|p| seen.insert(p.clone()));
```

Note this de-duplicates by path string, so `a.txt` and `./a.txt` are still treated as distinct — acceptable; canonicalising would add `stat` cost and resolve symlinks unexpectedly.

**Steps:**
1. Write a failing unit test in `src/unit_tests.rs` (`glob_tests`): two overlapping patterns over the temp dir, assert each file appears exactly once.
2. Write a failing test for a literal path supplied twice.
3. Implement the `retain` de-dup.
4. Confirm both new tests pass and the existing `test_multiple_patterns` (expects 3) still holds.

---

### M2 — `hash_file` stats then re-opens the file (TOCTOU + extra syscall)

**File:** `src/hash/digest_impl.rs`, lines 13–43, 69–72
**Symptom:** Every hash does at least two filesystem opens: `file_size()` calls `path.metadata()` (a `stat`), then the chunked path calls `File::open`, and the whole-file path calls `std::fs::read` (another open). Besides the wasted syscall, the size used to choose whole-vs-chunked is read at a different instant than the bytes, so a file truncated/extended in between is read correctly but classified on stale size.

**Remediation:** Open once, fstat via the handle, branch on that:

```rust
fn hash_file<D: Digest>(filename: impl AsRef<str>) -> anyhow::Result<Output<D>> {
    let mut file = File::open(filename.as_ref())?;
    let filesize = usize::try_from(file.metadata()?.len()).ok();

    if filesize.is_some_and(|size| size <= BUFFER_SIZE) {
        // small file: read whole, single digest call
        let mut data = Vec::with_capacity(filesize.unwrap_or(0));
        file.read_to_end(&mut data)?;
        return Ok(D::digest(&data));
    }

    #[allow(clippy::large_stack_arrays)]
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut hasher = D::new();
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize())
}
```

This lets `file_size` and `hash_file_whole` be deleted. Keep the existing 32-bit `try_from` comment.

**Steps:**
1. Existing KATs already cover whole-file, empty-file, and >32 KB chunked paths — they are the regression guard.
2. Refactor as above; delete `file_size` and `hash_file_whole`.
3. `cargo nextest run` — all hash KATs must still pass unchanged.

---

### M3 — Pre-release dependency (`blake2 0.11.0-rc.6`) shipped in a tagged release

**File:** `Cargo.toml`, line for `blake2`
**Symptom:** v1.6.3 depends on a release-candidate crate. RC crates can be yanked or change behaviour before final, and `cargo audit` advisories often lag pre-releases. The rest of the RustCrypto stack is on stable lines (`digest 0.11`, `sha2 0.11`, `sha3 0.12`).

**Remediation:** Track the blake2 0.11 final and pin to it once published; until then, document the deliberate RC pin in `Cargo.toml` with a comment and re-run `cargo audit` each session (already a CLAUDE.md habit). If a stable blake2 supporting `digest 0.11` is available, prefer it.

**Steps:**
1. `cargo search blake2` / check crates.io for a non-RC 0.11 release.
2. If available, bump and run the full pre-commit gate (build, clippy, fmt, tests).
3. If not, add `# pinned to RC until blake2 0.11 final ships; tracks digest 0.11` above the dependency.

---

## Non-Issues Verified (no action)

- **MT output ordering** — `par_iter().map().collect()` preserves input order; output is deterministic.
- **Broken-pipe handling** — propagated and swallowed cleanly in both ST and MT paths; `head` truncation no longer aborts.
- **U32/CRC32 coupling** — validated at the CLI layer and defensively re-checked in `hash_file_encoded` (4-byte guard); `call_hasher` hardcodes U32 for CRC32 regardless of the passed encoding.
- **Panics in production paths** — none; `ProgressStyle` templates fall back gracefully, `VERSION` uses `unwrap_or("?")`.
- **`unsafe`** — `#![forbid(unsafe_code)]` enforced crate-wide and via clippy flag.

---

## Suggested Order

| Priority | Items | Rationale |
|---|---|---|
| **Next** | M1, M2 | User-visible correctness/efficiency; both have existing or easy tests |
| **This session** | M3 | Dependency hygiene; re-check each `cargo audit` run |

## Checklist

- [ ] M1: De-duplicate collected paths (decide on observable-output change first; add tests, then `retain`)
- [ ] M2: Open file once, fstat via handle, delete `file_size` + `hash_file_whole`
- [ ] M3: Track blake2 0.11 final; pin off the RC or document the deliberate pin

## Verification gate (run after each change)

```
cargo build
cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
cargo fmt
gtimeout 60 cargo nextest run
```
