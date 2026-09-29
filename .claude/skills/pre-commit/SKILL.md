---
name: pre-commit
description: Run before committing changes - formats code, runs pedantic clippy, and runs tests
---

# Pre-commit Checks

Before every commit, run these in order — **all must pass cleanly**:

```
cargo build
cargo clippy --all-targets --all-features
cargo fmt
gtimeout 60 cargo nextest run
```

(test fallback: `gtimeout 60 cargo test`)

All must pass cleanly before committing.

**Security:** Run `cargo audit` at least once per working session.
