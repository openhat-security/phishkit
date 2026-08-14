#!/usr/bin/env bash
# Upload docs/media/*.mp4 to the docs-media GitHub Release (clobber).
# Requires: gh auth login
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VID_DIR="$ROOT/docs/media"
TAG="docs-media"
REPO="${GITHUB_REPOSITORY:-openhat-security/phishkit}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI required: https://cli.github.com/" >&2
  exit 1
fi

shopt -s nullglob
# Only the stable walkthrough filenames referenced by VitePress (not WDIO run leftovers).
files=("$VID_DIR"/walkthrough-*.mp4 "$VID_DIR"/walkthrough-*.webm)
if ((${#files[@]} == 0)); then
  echo "No walkthrough-*.mp4/webm in $VID_DIR — run: make docs-videos" >&2
  exit 1
fi

if ! gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "Docs walkthrough media" \
    --notes "Recorded walkthrough videos for the VitePress docs site. Regenerate with \`make update-video-documentation\` and re-upload with \`make publish-docs-videos\`." \
    --latest=false
fi

gh release upload "$TAG" "${files[@]}" --repo "$REPO" --clobber
echo "Uploaded ${#files[@]} file(s) to https://github.com/$REPO/releases/tag/$TAG"
