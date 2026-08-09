#!/usr/bin/env bash
# Start evilginx2 in LOCAL DRY-RUN mode for phishkit.
#
# This script is intentionally modeled after the original working
# evilginx2_testing/scripts/start_dryrun.sh that the user confirmed works perfectly.
#
# - Uses -developer mode (self-signed certs)
# - Prints the exact same style of instructions
# - Uses the same `phishlets hostname <phishlet> <base-dryrun-domain>` pattern
#   (NOT the full portal. + domain)
#
# Usage:
#   DRYRUN_DOMAIN="demo-cookie.local.phishkit" \
#   PHISHLET_NAME="demo-cookie" \
#   make evilginx-start
#
# Or directly:
#   ./evilginx/scripts/start_evilginx_dryrun.sh
set -euo pipefail
source "$(dirname "$0")/_env.sh"

[[ -x "$EVILGINX_BIN" ]] || die "evilginx binary missing. run: make build-evilginx"

DRYRUN_DOMAIN="${DRYRUN_DOMAIN:-demo-cookie.local.phishkit}"
PHISHLET_NAME="${PHISHLET_NAME:-demo-cookie}"

YELLOW=$'\e[1;33m'
RESET=$'\e[0m'

cat <<INFO

${YELLOW}===============================================================${RESET}
${YELLOW}  phishkit LOCAL DRY-RUN  (developer mode, self-signed certs)${RESET}
${YELLOW}===============================================================${RESET}

Defaults target the localhost demo suite (see demos/ and demos/).
Start the matching mock first when practising capture shapes:

  make demo-cookie     # cookie session on :9080  (phishlet: demo-cookie)
  make demo-firebase   # Firebase mock on :9081   (phishlet: demo-firebase)

Step 1 — add these lines to /etc/hosts BEFORE testing in your browser
(use the real landing subdomain from your phishlet — not an invented portal.):

  sudo tee -a /etc/hosts <<EOF
127.0.0.1   ${DRYRUN_DOMAIN}
127.0.0.1   api.${DRYRUN_DOMAIN}
# plus landing host if the site uses a subdomain, e.g.:
# 127.0.0.1   www.${DRYRUN_DOMAIN}
# 127.0.0.1   portal.${DRYRUN_DOMAIN}
EOF

Step 2 — once evilginx is up, paste this whole block at the ':' prompt:

  config domain ${DRYRUN_DOMAIN}
  config ipv4 external 127.0.0.1
  phishlets hostname ${PHISHLET_NAME} ${DRYRUN_DOMAIN}
  phishlets disable ${PHISHLET_NAME}
  phishlets enable ${PHISHLET_NAME}
  lures create ${PHISHLET_NAME}
  lures get-url 0

Step 3 — open the https:// URL it prints in a FRESH browser profile
(your normal browser's HSTS cache for the real site can block you).
Evilginx serves a SELF-SIGNED cert in -developer mode, so click
through the browser warning (Advanced -> Proceed). This is exactly
how the original evilginx2_testing dry-run worked.

NOTE: ${DRYRUN_DOMAIN} resolves to 127.0.0.1 (the phish side).
For client engagements, proxy_hosts.domain must be the REAL authorized
upstream and must resolve publicly — do not put that hostname in /etc/hosts.

When done: type exit or Ctrl+C.

INFO

read -r -p "Press enter to launch evilginx (Ctrl-C to abort)..." _

# Launch inside screen for background stability (phishkit preference)
# while still giving the exact same interactive instructions as the original.
SCREEN_NAME="phishkit-evilginx"

# Clean up any old session
if screen -ls | grep -q "$SCREEN_NAME"; then
    screen -S "$SCREEN_NAME" -X quit 2>/dev/null || true
    sleep 1
fi

# Aggressive port cleanup
for port in 443 80 53; do
    pids=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null || true)
    for pid in $pids; do
        cmd=$(ps -p "$pid" -o comm= 2>/dev/null || true)
        if [[ "$cmd" == *evilginx* ]]; then
            kill "$pid" 2>/dev/null || true
            sleep 0.3
            kill -9 "$pid" 2>/dev/null || true
        fi
    done
done

mkdir -p "$EVILGINX_DATA_DIR"

log "starting evilginx inside screen session '$SCREEN_NAME'"
log "  domain   : $DRYRUN_DOMAIN"
log "  phishlet : $PHISHLET_NAME"

screen -dmS "$SCREEN_NAME" \
    "$EVILGINX_BIN" \
        -p "$PHISHLETS_DIR" \
        -t "$REDIRECTORS_DIR" \
        -c "$EVILGINX_DATA_DIR" \
        -developer \
        -debug \
    > "$EVILGINX_LOG" 2>&1

sleep 2

pid=$(pgrep -f "evilginx.*$EVILGINX_DATA_DIR" | head -1 || true)
if [[ -n "$pid" ]]; then
    echo "$pid" > "$EVILGINX_PID"
    log "evilginx running inside screen (pid $pid)"
fi

log "debug log: $EVILGINX_LOG"
log "attach with: screen -r $SCREEN_NAME"
log "detach with: Ctrl+A then D"

echo
echo "After attaching, paste the exact block from Step 2 above."
echo "Open the printed https:// lure URL and click through the self-signed cert warning."
