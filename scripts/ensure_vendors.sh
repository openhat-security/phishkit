#!/usr/bin/env bash
# Initialize or update vendor/ git submodules (evilginx2).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KIT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

command -v git >/dev/null 2>&1 || { echo "git is required"; exit 1; }

cd "$KIT_ROOT"

if [[ ! -f .gitmodules ]]; then
    echo "[vendor] ERROR: .gitmodules not found — vendor submodules are not configured." >&2
    exit 1
fi

echo "[vendor] syncing submodules..."
git submodule sync --recursive

echo "[vendor] initializing/updating submodules..."
git submodule update --init --recursive

EVIL_DIR="$KIT_ROOT/vendor/evilginx2"

if [[ ! -d "$EVIL_DIR" ]]; then
    echo "[vendor] ERROR: expected submodule missing: $EVIL_DIR" >&2
    exit 1
fi

echo "[vendor] caching Go modules..."
if [[ -f "$EVIL_DIR/go.mod" ]]; then
    (cd "$EVIL_DIR" && go mod download 2>&1 | tail -5 || true)
fi

echo "[vendor] submodules ready:"
echo "  vendor/evilginx2 @ $(git -C "$EVIL_DIR" rev-parse --short HEAD)"
