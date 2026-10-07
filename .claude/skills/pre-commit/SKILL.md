---
name: pre-commit
description: Run before committing changes (git commit, jj commit, jj describe when finalising, jj squash) - checks formatting, runs pedantic clippy, builds and runs tests
---

# Pre-commit checks

Run from the repo root:

```bash
scripts/pre-commit.sh
```

It works in jj and plain git repos. It inspects the changes in `@` (jj) or against `HEAD` (git), skips the checks when no `.rs`, `Cargo.toml` or `Cargo.lock` file changed, and otherwise runs `scripts/checks.sh`: `cargo fmt --check`, `cargo build --all-targets`, `cargo clippy --all-targets --all-features` and `cargo nextest run` under a timeout. Lint levels live in `Cargo.toml` `[lints]`.

On a `cargo fmt --check` failure, run `cargo fmt` and rerun the script. All checks must pass with zero warnings or errors before committing.

**Security:** run `cargo audit` at least once per working session.
