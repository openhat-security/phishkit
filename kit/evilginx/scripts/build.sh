#!/usr/bin/env bash
# Build evilginx2 from the local source tree into this kit's run/ directory.
set -euo pipefail
source "$(dirname "$0")/_env.sh"

# Auto-clone upstream evilginx2 submodule if missing.
# This makes phishkit fully standalone.
if [[ ! -d "$EVILGINX_SRC" ]]; then
    log "evilginx2 not found in vendor/ — running shared ensure script..."
    "$KIT_ROOT/scripts/ensure_vendors.sh"
fi

command -v go >/dev/null 2>&1 || die "go toolchain is required"

log "building evilginx2 from $EVILGINX_SRC"
pushd "$EVILGINX_SRC" >/dev/null

# Use a robust build command that works on a fresh clone (with or without vendor dir).
# The old "main.go -mod=vendor" form is brittle on freshly cloned trees.
if ! go build -mod=vendor -o "$EVILGINX_BIN" . 2>/dev/null; then
    log "build with -mod=vendor failed, falling back to normal module build..."
    go build -o "$EVILGINX_BIN" .
fi

popd >/dev/null

log "built: $EVILGINX_BIN"
"$EVILGINX_BIN" -h 2>&1 | head -15 || true
