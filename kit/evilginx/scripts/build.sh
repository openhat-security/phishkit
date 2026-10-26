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

# Force the linker to emit a build ID. On Mach-O (Darwin) the Go internal
# linker only writes an LC_UUID load command when a build ID is present; older
# toolchains (e.g. the go1.22 we pinned in CI) otherwise produce a binary with
# NO LC_UUID, and modern macOS dyld refuses to run it:
#   dyld: missing LC_UUID load command
# `-B gobuildid` derives the build ID from Go's own build ID, so it is stable,
# toolchain-version independent, and needs no C compiler / external linker.
BUILD_LDFLAGS="-B gobuildid"

log "building evilginx2 from $EVILGINX_SRC"
pushd "$EVILGINX_SRC" >/dev/null

# Use a robust build command that works on a fresh clone (with or without vendor dir).
# The old "main.go -mod=vendor" form is brittle on freshly cloned trees.
if ! go build -mod=vendor -ldflags="$BUILD_LDFLAGS" -o "$EVILGINX_BIN" . 2>/dev/null; then
    log "build with -mod=vendor failed, falling back to normal module build..."
    go build -ldflags="$BUILD_LDFLAGS" -o "$EVILGINX_BIN" .
fi

popd >/dev/null

# Guard: a Darwin evilginx without LC_UUID is a non-runnable binary (dyld will
# reject it). Fail the build here so a broken artifact never ships in a release.
if [[ "$(go env GOOS)" == "darwin" ]]; then
    if command -v otool >/dev/null 2>&1; then
        if ! otool -l "$EVILGINX_BIN" | grep -q LC_UUID; then
            die "evilginx Darwin build is missing LC_UUID — dyld would refuse to run it. Expected -ldflags=-B gobuildid on a native macOS toolchain to add it."
        fi
        log "verified LC_UUID present in Darwin evilginx"
    else
        log "WARNING: otool not found; cannot verify LC_UUID on this Darwin build"
    fi
fi

log "built: $EVILGINX_BIN"
"$EVILGINX_BIN" -h 2>&1 | head -15 || true
