---
name: pre-commit
description: Run before committing changes - formats code, runs pedantic clippy, and runs tests
---

# Pre-commit Checks

Run after every code change, before committing:

1. `cargo fmt` — format all code
2. `cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code` — pedantic lint
3. `gtimeout 60 cargo nextest run` — full test suite (fallback: `gtimeout 60 cargo test`)

All must pass cleanly before committing.

**Security:** Run `cargo audit` at least once per working session.
