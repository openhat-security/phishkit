#!/usr/bin/env bash
# Stage the runtime "kit root" tree that a packaged phishkit (desktop .app or
# CLI tarball) needs so `kit_root()` resolves without a manual PHISHKIT_ROOT.
#
# Usage: scripts/stage_kit_root.sh <dest-dir>
#
# The destination becomes a valid kit root: it contains kit/evilginx/phishlets
# (what validate_kit_root checks), the built evilginx binary (if present), the
# redirectors evilginx is launched with, the community phishlet index, the
# top-level helper scripts the engine shells out to, and a Makefile (the shell
# harness's _env.sh sanity-checks for it). Runtime state and caches are excluded.
#
# Uses cp -R (not rsync) for portability: macOS CI runners ship openrsync, whose
# --relative/--exclude handling differs from GNU rsync.
set -euo pipefail

DEST="${1:?usage: stage_kit_root.sh <dest-dir>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rm -rf "$DEST"
mkdir -p "$DEST"

copy() {
  # copy <relative-src> into DEST preserving tree + perms (keeps evilginx +x)
  local rel="$1"
  [ -e "$ROOT/$rel" ] || { echo "stage_kit_root: skip missing $rel" >&2; return 0; }
  mkdir -p "$DEST/$(dirname "$rel")"
  cp -R "$ROOT/$rel" "$DEST/$rel"
}

copy kit
copy scripts
copy vendor/evilginx2/redirectors
copy vendor/community-phishlets
# _env.sh requires a Makefile at the kit root; ship it so the shell harness runs.
cp "$ROOT/Makefile" "$DEST/Makefile"

# Drop runtime state + caches that cp -R may have pulled in.
rm -rf "$DEST/kit/evilginx/run/data" "$DEST/kit/evilginx/run/redirectors"
find "$DEST" -name '__pycache__' -type d -prune -exec rm -rf {} + 2>/dev/null || true
find "$DEST" -name '*.pyc' -delete 2>/dev/null || true
find "$DEST" -name '.DS_Store' -delete 2>/dev/null || true

if [ -f "$DEST/kit/evilginx/run/evilginx" ]; then
  chmod +x "$DEST/kit/evilginx/run/evilginx"
  echo "stage_kit_root: staged with evilginx binary -> $DEST"
else
  echo "stage_kit_root: WARNING staged WITHOUT evilginx binary (run make build-evilginx first) -> $DEST" >&2
fi
