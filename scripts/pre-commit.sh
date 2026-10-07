#!/usr/bin/env bash
# pre-commit.sh — run this project's required checks before committing.
#
# jj: run before `jj commit`, `jj describe` (finalising) and `jj squash`.
#     Inspects the changes in @.
# git: install once as a hook, or run directly. Inspects changes against HEAD.
#   ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
#
# Skips the checks when no Rust or Cargo file changed.

set -euo pipefail

is_rust_path() {
    [[ "$1" == *.rs || "$1" == "Cargo.toml" || "$1" == "Cargo.lock" ]]
}

changed_paths() {
    if jj --ignore-working-copy root > /dev/null 2>&1; then
        jj diff --name-only -r @
    else
        git diff HEAD --name-only -z | tr '\0' '\n'
    fi
}

# Run from the repo root; works when invoked via the .git/hooks symlink.
root="$(jj --ignore-working-copy root 2> /dev/null || git rev-parse --show-toplevel)"
cd "${root}"

should_run=false
while IFS= read -r path; do
    if is_rust_path "${path}"; then
        should_run=true
        break
    fi
done < <(changed_paths)

if [[ "${should_run}" != true ]]; then
    echo "==> No modified Rust or Cargo files detected, skipping."
    exit 0
fi

echo "==> Running HashRust pre-commit checks..."
"${root}/scripts/checks.sh"
echo "==> All checks passed."
