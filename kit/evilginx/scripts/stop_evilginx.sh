#!/usr/bin/env bash
# Stop the screen-based evilginx managed by phishkit
set -euo pipefail
source "$(dirname "$0")/_env.sh"

SCREEN_NAME="phishkit-evilginx"

if screen -ls | grep -q "$SCREEN_NAME"; then
    log "stopping screen session $SCREEN_NAME"
    screen -S "$SCREEN_NAME" -X quit 2>/dev/null || true
    sleep 1
fi

# Also kill any stray evilginx processes that might be left
pkill -f "evilginx.*$EVILGINX_DATA_DIR" 2>/dev/null || true

rm -f "$EVILGINX_PID"
log "evilginx stopped"
