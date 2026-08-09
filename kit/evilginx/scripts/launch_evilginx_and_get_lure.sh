#!/usr/bin/env bash
#
# Configure evilginx for the requested profile and print the lure URL.
# Delegates to configure_lure.py (pexpect REPL driver — works on macOS).
#
# Usage:
#   DRYRUN_DOMAIN=portal.client.phishkit
#   PHISHLET_NAME=portal-client-com-portal
#   ./evilginx/scripts/launch_evilginx_and_get_lure.sh
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_env.sh"

DRYRUN_DOMAIN="${DRYRUN_DOMAIN:-example-phish.local.test}"
PHISHLET_NAME="${PHISHLET_NAME:-generic}"
PROFILE_ID="${PROFILE_ID:-}"

PYTHON="${PYTHON:-python3}"
if [[ -x "$KIT_ROOT/venv/bin/python3" ]]; then
    PYTHON="$KIT_ROOT/venv/bin/python3"
fi

ARGS=(--phishlet "$PHISHLET_NAME" --dryrun-domain "$DRYRUN_DOMAIN")
if [[ -n "$PROFILE_ID" ]]; then
    ARGS+=(--profile-id "$PROFILE_ID")
fi
exec "$PYTHON" "$SCRIPT_DIR/configure_lure.py" "${ARGS[@]}"
