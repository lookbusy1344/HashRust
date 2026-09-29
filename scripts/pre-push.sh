#!/usr/bin/env bash
# pre-push.sh — format, build, lint and test unpushed jj revisions.
#
# Default: the tip only (the newest non-empty mutable revision in ::@).
# --full: every non-empty mutable revision in ::@, each in its own checkout
# via `jj run`.
#
# Run before `jj git push` or moving a shared bookmark. Lint levels live in
# Cargo.toml [lints], so plain `cargo clippy` enforces them.

set -euo pipefail

readonly TEST_TIMEOUT_SECONDS=60
readonly STACK='mutable() & ::@ ~ empty()'
readonly TIP="heads(${STACK})"
readonly CHECKS="cargo fmt --check \
&& cargo build --all-targets \
&& cargo clippy --all-targets --all-features \
&& gtimeout ${TEST_TIMEOUT_SECONDS} cargo nextest run"

# Inline so formatting does not depend on unversioned .jj/repo/config.toml.
# rustfmt on stdin ignores Cargo.toml, so the edition is passed explicitly.
readonly RUSTFMT_CONFIG=(
    --config 'fix.tools.rustfmt.command=["rustfmt", "--emit", "stdout", "--edition", "2024"]'
    --config 'fix.tools.rustfmt.patterns=["glob:\"**/*.rs\""]'
    --config 'fix.tools.rustfmt.enabled=true'
)

usage() {
    echo "Usage: $0 [--full|-f]"
    echo "  --full, -f  Format and check every mutable revision, not only the tip."
}

full=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --full | -f) full=true; shift ;;
        -h | --help) usage; exit 0 ;;
        *) echo "Error: unknown argument '$1'" >&2; usage >&2; exit 1 ;;
    esac
done

# Git-only checkouts rely on CI.
if ! root="$(jj --ignore-working-copy root 2> /dev/null)"; then
    echo "==> jj not installed or not a jj repository, skipping pre-push checks."
    exit 0
fi
cd "${root}"

stack="$(jj log --no-graph -r "${STACK}" -T 'change_id ++ "\n"')"
if [[ -z "${stack}" ]]; then
    echo "==> No non-empty mutable revisions in ${STACK}, nothing to check."
    exit 0
fi

if [[ "${full}" == true ]]; then
    echo "==> Formatting ${STACK}"
    jj "${RUSTFMT_CONFIG[@]}" fix -s "roots(${STACK})"
    echo "==> Checking each revision in ${STACK}"
    jj run --root --ignore-changes -r "${STACK}" -- bash -c "${CHECKS}"
else
    # Checks run in the working copy; when @ is empty its tree is the tip's.
    echo "==> Formatting tip ${TIP}"
    jj "${RUSTFMT_CONFIG[@]}" fix -s "${TIP}"
    echo "==> Checking tip $(jj log --no-graph -r "${TIP}" -T 'change_id.short() ++ " " ++ coalesce(description.first_line(), "(no description)")')"
    bash -c "${CHECKS}"
fi

echo "==> All checks passed."
