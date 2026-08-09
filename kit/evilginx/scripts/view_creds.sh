#!/usr/bin/env bash
# Dump captured credentials + tokens from evilginx's data.db.
#
# Usage examples:
#   view_creds.sh
#   view_creds.sh --full
#   view_creds.sh --id 3
#   view_creds.sh --json
set -euo pipefail
source "$(dirname "$0")/_env.sh"

DB="$EVILGINX_DATA_DIR/data.db"
[[ -f "$DB" ]] || die "no db at $DB (has evilginx ever run?)"

command -v python3 >/dev/null 2>&1 || die "python3 required"

EVILGINX_DB="$DB" exec python3 "$(dirname "$0")/../scripts/view_creds.py" "$@"
