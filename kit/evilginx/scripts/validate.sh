#!/usr/bin/env bash
# Validate a phishlet YAML by launching evilginx briefly and checking for parse errors.
set -euo pipefail
source "$(dirname "$0")/_env.sh"

[[ -x "$EVILGINX_BIN" ]] || die "evilginx binary missing. run: make build-evilginx"

PHISHLET="${1:-${PHISHLET:-demo-cookie}}"
YAML="$PHISHLETS_DIR/${PHISHLET}.yaml"
[[ -f "$YAML" ]] || die "phishlet not found: $YAML"

log "validating $YAML"

TMP_CFG="$(mktemp -d)"
trap 'rm -rf "$TMP_CFG"' EXIT

set +e
out=$(printf 'exit\n' | "$EVILGINX_BIN" \
    -p "$PHISHLETS_DIR" \
    -t "$REDIRECTORS_DIR" \
    -c "$TMP_CFG" \
    -developer 2>&1)
rc=$?
set -e

if echo "$out" | grep -qiE "failed to load phishlet.*${PHISHLET}|error.*${PHISHLET}\\.yaml"; then
    echo "$out" | grep -iE "failed to load phishlet|error.*${PHISHLET}\\.yaml"
    die "phishlet failed to parse"
fi

log "phishlet '${PHISHLET}' parsed without errors"
echo "$out" | grep -E "(loaded phishlets|created phishlet|hostname|phishlet|error)" | head -10 || true
