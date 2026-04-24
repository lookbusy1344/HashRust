---
name: require-pre-commit-checks
enabled: true
event: bash
pattern: git\s+commit
action: warn
---

**Pre-commit check required.**

Before proceeding, run `git diff --cached --name-only` to inspect staged files.

**If all staged files are documentation only** (`.md` files, `docs/`, `README`):
- No build or test checks required. Proceed with the commit.

**If any Rust source files are staged** (`.rs`, `Cargo.toml`, `Cargo.lock`):
- You must run ALL of the following in order before committing:

```bash
cargo build
cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
cargo fmt
gtimeout 60 cargo nextest run
```

All steps must pass with zero warnings or errors.

If you have already completed all checks in this session and they passed, you may proceed.
