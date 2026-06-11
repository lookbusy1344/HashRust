#!/bin/bash
# Last change: 2026-05-19 08:22 BST — added Node.js / pnpm / npm / yarn
# cache paths in the companion .sb profile so `pnpm install`, vite,
# vitest, esbuild and node-gyp work inside the sandbox.  Global bin
# dirs stay read-only — symmetric with cargo/go/dotnet/pipx.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ ! -f "$SCRIPT_DIR/opencode_sandbox.sb" ]]; then
    printf 'error: opencode_sandbox.sb not found in %s\n' "$SCRIPT_DIR" >&2
    exit 1
fi

if [[ ! -f "$SCRIPT_DIR/opencode_sandbox.json" ]]; then
    printf 'error: opencode_sandbox.json not found in %s\n' "$SCRIPT_DIR" >&2
    exit 1
fi

# Point opencode at the sandbox-only config.  Deliberately NOT named
# opencode.json so that launching opencode outside this wrapper falls
# back to the (stricter) opencode defaults instead of silently picking
# up the permissive ruleset that assumes Seatbelt is enforcing the
# filesystem boundary.
export OPENCODE_CONFIG="$SCRIPT_DIR/opencode_sandbox.json"

# Resolve the enclosing git repo root so writes to .git/ (index, refs,
# objects, hooks output) are allowed even when PROJECT is a subdirectory
# of a larger repo.  Fall back to PROJECT if we're not inside a git repo.
if GITROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null)"; then
    :
else
    GITROOT="$SCRIPT_DIR"
fi

# Optional: prepend a shim directory if one is checked in.  Currently used
# by SwiftPM-invoking tools (swift build, swift test, periphery) to inject
# --disable-sandbox so SwiftPM's inner sandbox-exec call nests under this
# Seatbelt profile.  Other ecosystems (dotnet, cargo, go, python) don't
# need shims; the directory is absent in those projects and the prepend
# becomes a no-op.
if [[ -d "$SCRIPT_DIR/scripts/sandbox-shims" ]]; then
    export PATH="$SCRIPT_DIR/scripts/sandbox-shims:$PATH"
fi

exec sandbox-exec \
    -f "$SCRIPT_DIR/opencode_sandbox.sb" \
    -D PROJECT="$SCRIPT_DIR" \
    -D GITROOT="$GITROOT" \
    -D HOME="$HOME" \
    -- opencode "$@"
