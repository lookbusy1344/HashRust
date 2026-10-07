---
name: require-pre-commit-checks
enabled: true
event: bash
pattern: git\s+commit|jj\s+(commit|describe|squash)
action: warn
---

**Pre-commit check required.**

Run `scripts/pre-commit.sh` before committing. It works in jj and plain git repos, skips the checks when no `.rs`, `Cargo.toml` or `Cargo.lock` file changed, and otherwise runs fmt check, build, clippy and nextest.

It must pass with zero warnings or errors. If it already passed in this session and nothing changed since, proceed.
