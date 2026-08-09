#!/usr/bin/env bash
# Start evilginx2 in LOCAL DRY-RUN mode (self-signed certs).
#
# This is ONLY for validating your phishlet against yourself.
# For any real engagement you need a public VPS + registered lookalike domain.
set -euo pipefail
source "$(dirname "$0")/_env.sh"

[[ -x "$EVILGINX_BIN" ]] || die "evilginx binary missing. run: make build-evilginx"

# Safe throwaway domain for local testing. Never collides with real HSTS preload lists.
DRYRUN_DOMAIN="${DRYRUN_DOMAIN:-example-phish.local.test}"

check_port_free 443
check_port_free 80
check_port_free 53 || warn "evilginx wants DNS port 53; try 'sudo' if this fails."

YELLOW=$'\e[1;33m'
RESET=$'\e[0m'

cat <<INFO

${YELLOW}============================================================${RESET}
${YELLOW}  evilginx2 LOCAL DRY-RUN  (developer mode, self-signed)${RESET}
${YELLOW}============================================================${RESET}

This is for testing your customized phishlet against YOURSELF only.

Step 1 — add these lines to /etc/hosts (landing sub = real site host, not invent portal.):
  sudo tee -a /etc/hosts <<EOF
127.0.0.1   ${DRYRUN_DOMAIN}
127.0.0.1   api.${DRYRUN_DOMAIN}
EOF

Step 2 — once evilginx starts, paste at the ':' prompt:

  config domain ${DRYRUN_DOMAIN}
  config ipv4 external 127.0.0.1
  phishlets hostname YOUR_PHISHLET_NAME ${DRYRUN_DOMAIN}
  phishlets enable YOUR_PHISHLET_NAME
  lures create YOUR_PHISHLET_NAME
  lures get-url 0

Step 3 — open the printed URL in a FRESH browser profile.
Accept the self-signed cert warning and test the login flow.

When finished: type 'exit' at the evilginx prompt.

INFO

read -r -p "Press enter to launch (Ctrl-C to abort)..." _

"$EVILGINX_BIN" \
    -p "$PHISHLETS_DIR" \
    -t "$REDIRECTORS_DIR" \
    -c "$EVILGINX_DATA_DIR" \
    -developer \
    -debug
