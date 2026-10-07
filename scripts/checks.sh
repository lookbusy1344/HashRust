#!/usr/bin/env bash
# checks.sh — fmt check, build, clippy and nextest in the current directory.
#
# Shared by pre-commit.sh and pre-push.sh. Runs against whatever tree the
# caller has checked out, so pre-push.sh can call it from temporary checkouts.
# Lint levels live in Cargo.toml [lints], so plain `cargo clippy` enforces them.

set -euo pipefail

readonly TEST_TIMEOUT_SECONDS=60

run() {
    echo "==> $*"
    "$@"
}

# gtimeout on macOS (coreutils), timeout on Linux.
timeout_cmd="$(command -v gtimeout || command -v timeout || true)"
if [[ -n "${timeout_cmd}" ]]; then
    test_prefix=("${timeout_cmd}" "${TEST_TIMEOUT_SECONDS}")
else
    test_prefix=()
fi

run cargo fmt --check
run cargo build --all-targets
run cargo clippy --all-targets --all-features
run ${test_prefix[@]+"${test_prefix[@]}"} cargo nextest run
