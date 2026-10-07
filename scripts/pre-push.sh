#!/usr/bin/env bash
# pre-push.sh — format, build, lint and test unpushed revisions.
#
# jj:
#   Default: format the tip (newest non-empty mutable revision in ::@) with
#   `jj fix`, then check it in the working copy.
#   --full: format every non-empty mutable revision in ::@ and check each in
#   its own checkout via `jj run`.
# git:
#   Default: check HEAD, in place when the working tree is clean, otherwise
#   in a temporary clone of HEAD.
#   --full: check each commit in @{upstream}..HEAD in a temporary clone.
#   Formatting is checked, not applied: git cannot rewrite pushed-to-be
#   commits safely here.
#   Install as a hook: ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push
#
# Run before `jj git push`, `git push` or moving a shared bookmark.

set -euo pipefail

readonly STACK='mutable() & ::@ ~ empty()'
readonly TIP="heads(${STACK})"

# Inline so formatting does not depend on unversioned .jj/repo/config.toml.
# rustfmt on stdin ignores Cargo.toml, so the edition is passed explicitly.
readonly RUSTFMT_CONFIG=(
    --config 'fix.tools.rustfmt.command=["rustfmt", "--emit", "stdout", "--edition", "2024"]'
    --config 'fix.tools.rustfmt.patterns=["glob:\"**/*.rs\""]'
    --config 'fix.tools.rustfmt.enabled=true'
)

usage() {
    echo "Usage: $0 [--full|-f]"
    echo "  --full, -f  Check every unpushed revision, not only the tip."
}

full=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --full | -f) full=true; shift ;;
        -h | --help) usage; exit 0 ;;
        -*) echo "Error: unknown argument '$1'" >&2; usage >&2; exit 1 ;;
        # git passes <remote> <url> when run as a pre-push hook.
        *) shift ;;
    esac
done

describe_git() {
    git log -1 --format='%h %s' "$1"
}

# Check one git commit in a throwaway clone. --shared borrows the object
# store, and a real repo keeps build-time `git describe` working. A shared
# target dir reuses compiled dependencies across clones.
check_git_export() {
    local rev="$1" dir
    dir="$(mktemp -d)"
    git clone --quiet --shared --no-checkout "${root}" "${dir}"
    git -C "${dir}" checkout --quiet --detach "${rev}"
    echo "==> Checking $(describe_git "${rev}")"
    local status=0
    (cd "${dir}" && "${checks}") || status=$?
    rm -rf "${dir}"
    return "${status}"
}

run_jj() {
    local stack
    stack="$(jj log --no-graph -r "${STACK}" -T 'change_id ++ "\n"')"
    if [[ -z "${stack}" ]]; then
        echo "==> Nothing to check: no non-empty mutable revisions in ${STACK}."
        return
    fi

    if [[ "${full}" == true ]]; then
        echo "==> Formatting ${STACK}"
        jj "${RUSTFMT_CONFIG[@]}" fix -s "roots(${STACK})"
        echo "==> Checking each revision in ${STACK}"
        jj run --root --ignore-changes -r "${STACK}" -- "${checks}"
    else
        # Checks run in the working copy; when @ is empty its tree is the tip's.
        echo "==> Formatting tip ${TIP}"
        jj "${RUSTFMT_CONFIG[@]}" fix -s "${TIP}"
        echo "==> Checking tip $(jj log --no-graph -r "${TIP}" -T 'change_id.short() ++ " " ++ coalesce(description.first_line(), "(no description)")')"
        "${checks}"
    fi
}

run_git() {
    if [[ "${full}" == true ]]; then
        local upstream revs
        if ! upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2> /dev/null)"; then
            echo "Error: --full needs an upstream branch for $(git branch --show-current)." >&2
            exit 1
        fi
        revs="$(git rev-list --reverse "${upstream}..HEAD")"
        if [[ -z "${revs}" ]]; then
            echo "==> Nothing to check: no commits in ${upstream}..HEAD."
            return
        fi
        while IFS= read -r rev; do
            check_git_export "${rev}"
        done <<< "${revs}"
    elif [[ -z "$(git status --porcelain)" ]]; then
        echo "==> Checking HEAD $(describe_git HEAD)"
        "${checks}"
    else
        check_git_export "$(git rev-parse HEAD)"
    fi
}

if root="$(jj --ignore-working-copy root 2> /dev/null)"; then
    mode=jj
elif root="$(git rev-parse --show-toplevel 2> /dev/null)"; then
    mode=git
else
    echo "Error: not inside a jj or git repository." >&2
    exit 1
fi
cd "${root}"

readonly checks="${root}/scripts/checks.sh"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${root}/target}"

"run_${mode}"
echo "==> All checks passed."
