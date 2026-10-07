#!/usr/bin/env bash
# Agent pre-tool hook: run scripts/pre-push.sh before any push.
#
# Shared by Claude Code (.claude/settings.json) and Codex (.codex/hooks.json).
# Both send the PreToolUse call as JSON on stdin with the shell command at
# .tool_input.command, and block the call on exit 2 with stderr as the reason.
# On success it prints JSON: systemMessage for the user, additionalContext for
# the agent.

set -euo pipefail

readonly BLOCK_EXIT_CODE=2
readonly LOG_TAIL_LINES=60
readonly PUSH_PATTERN='(^|[^[:alnum:]_-])(jj[[:space:]]+git[[:space:]]+push|jj[[:space:]]+pmain|git[[:space:]]+push)([^[:alnum:]_-]|$)'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PROJECT_DIR

command="$(jq -r '.tool_input.command // empty')"
grep -Eq "${PUSH_PATTERN}" <<< "${command}" || exit 0

log="$(mktemp)"
trap 'rm -f "${log}"' EXIT

if ! "${PROJECT_DIR}/scripts/pre-push.sh" > "${log}" 2>&1; then
    echo "Push blocked: scripts/pre-push.sh failed. Fix the failures and retry." >&2
    tail -n "${LOG_TAIL_LINES}" "${log}" >&2
    exit "${BLOCK_EXIT_CODE}"
fi

# grep exits 1 on no match; the no-op path runs no tests.
checked="$({ grep -E '^==> (Checking|Nothing to check)' "${log}" || true; } | sed 's/^==> //' | paste -sd ';' -)"
tests="$({ grep -Eo '[0-9]+ tests run: .*' "${log}" || true; } | tail -n 1)"
message="Pre-push checks passed: ${checked}${tests:+ (${tests})}"

jq -n --arg message "${message}" '{
    systemMessage: $message,
    hookSpecificOutput: {hookEventName: "PreToolUse", additionalContext: $message}
}'
