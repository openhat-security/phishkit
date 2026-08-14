#!/usr/bin/env bash
# Walkthrough MP4s must not be uploaded to GitHub. They grow quickly and
# count against release bandwidth. Generate locally instead:
#   VIDEO=1 make test-integration
#   make update-video-documentation
# Later hosting, if any, should be a CDN or unlisted YouTube — not a GitHub
# Release tag. See docs/guide/walkthrough.md.
set -euo pipefail

echo "Refused: do not publish walkthrough videos to GitHub." >&2
echo "MP4s stay local (docs/media/, tests/integration/artifacts/, both gitignored)." >&2
echo "See docs/guide/walkthrough.md" >&2
exit 1
