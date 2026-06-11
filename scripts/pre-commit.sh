#!/usr/bin/env bash
# pre-commit.sh — run required HashRust checks before committing.
#
# Install:
#   ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
#
# Can also be run directly:
# if [ ! -x "scripts/pre-commit.sh" ]; then
#      echo "Missing executable scripts/pre-commit.sh" >&2
#      exit 1
# fi

# exec ./scripts/pre-commit.sh

set -euo pipefail

# Resolve the real script path before computing directories — dirname "$0" follows
# the symlink path (.git/hooks/) when invoked as a hook, not the script's location.
resolve_script_path() {
    local source_path="$1"

    while [[ -L "${source_path}" ]]; do
        local source_dir
        source_dir="$(cd -P "$(dirname "${source_path}")" && pwd)"
        source_path="$(readlink "${source_path}")"

        if [[ "${source_path}" != /* ]]; then
            source_path="${source_dir}/${source_path}"
        fi
    done

    local resolved_dir
    resolved_dir="$(cd -P "$(dirname "${source_path}")" && pwd)"
    printf '%s/%s\n' "${resolved_dir}" "$(basename "${source_path}")"
}

readonly TEST_TIMEOUT_SECONDS=60

REAL_SCRIPT="$(resolve_script_path "$0")"
SCRIPT_DIR="$(cd "$(dirname "${REAL_SCRIPT}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_DIR}"

run() {
    echo "==> $*"
    "$@"
}

is_documentation_path() {
    local path="$1"
    [[ "${path}" == docs/* ]] || [[ "${path}" == *.md ]] || [[ "${path}" == README ]]
}

is_rust_check_path() {
    local path="$1"
    [[ "${path}" == *.rs ]] || [[ "${path}" == "Cargo.toml" ]] || [[ "${path}" == "Cargo.lock" ]]
}

has_staged_paths=false
doc_only=true
should_run=false
while IFS= read -r -d '' staged_path; do
    has_staged_paths=true

    if ! is_documentation_path "${staged_path}"; then
        doc_only=false
    fi

    if is_rust_check_path "${staged_path}"; then
        should_run=true
    fi
done < <(git -C "${PROJECT_DIR}" diff HEAD --name-only -z)

if [[ "${has_staged_paths}" != true ]]; then
    echo "==> No staged files detected, skipping pre-commit checks."
    exit 0
fi

if [[ "${doc_only}" == true ]]; then
    echo "==> Documentation-only commit detected, skipping pre-commit checks."
    exit 0
fi

if [[ "${should_run}" != true ]]; then
    echo "==> No Rust source, Cargo.toml, or Cargo.lock files staged, skipping pre-commit checks."
    exit 0
fi

echo "==> Running HashRust pre-commit checks..."

run cargo build
run cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
run cargo fmt --check

run gtimeout "${TEST_TIMEOUT_SECONDS}" cargo nextest run

echo "==> All required checks passed."
