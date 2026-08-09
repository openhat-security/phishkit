#!/usr/bin/env bash
# NON-INTERACTIVE version of start_dryrun.sh.
# Intended to be driven by automation (pexpect / Python).
# It still prints the exact commands that will be fed, then execs evilginx.
#
# The caller is responsible for feeding the config lines at the ':' prompt
# and capturing the output of "lures get-url 0".
set -euo pipefail
source "$(dirname "$0")/_env.sh"

[[ -x "$EVILGINX_BIN" ]] || die "evilginx binary missing. run: make build-evilginx"

DRYRUN_DOMAIN="${DRYRUN_DOMAIN:-example-phish.local.test}"
PHISHLET_NAME="${PHISHLET_NAME:-generic}"

check_port_free 443
check_port_free 80
check_port_free 53 || warn "evilginx wants DNS port 53; try 'sudo' if this fails."

YELLOW=$'\e[1;33m'
RESET=$'\e[0m'

# These are the lines the automation layer will send after the ':' prompt appears.
cat <<COMMANDS

${YELLOW}=== HEADLESS EVILGINX DRY-RUN ===${RESET}
Domain     : ${DRYRUN_DOMAIN}
Phishlet   : ${PHISHLET_NAME}

Automation will send these commands:
  config domain ${DRYRUN_DOMAIN}
  config ipv4 external 127.0.0.1
  phishlets hostname ${PHISHLET_NAME} ${DRYRUN_DOMAIN}
  phishlets enable ${PHISHLET_NAME}
  lures create ${PHISHLET_NAME}
  lures get-url 0

COMMANDS

# No interactive prompt — just launch.
# The parent (Python + pexpect) will handle the REPL.
exec "$EVILGINX_BIN" \
    -p "$PHISHLETS_DIR" \
    -t "$REDIRECTORS_DIR" \
    -c "$EVILGINX_DATA_DIR" \
    -developer \
    -debug
