#!/usr/bin/env bash
# Removed: use the native CLI / desktop mail engine instead.
set -euo pipefail
echo "scripts/send_campaign_with_lure.sh has been removed." >&2
echo "Use:  make cli && ./target/release/phishkit wiz send" >&2
echo "Or:   make desktop" >&2
exit 1
