#!/usr/bin/env bash
# =============================================================================
# AiTM demo smoke test (unprivileged) — the cookie capture surface end to end.
#
# This exercises the exact failure the audit called out in C1.4: the demo needs a
# *reachable HTTPS upstream on a host distinct from the lure*. Here we stand that
# upstream up for real (self-signed TLS) and drive the cookie login -> session
# -> dashboard flow over HTTPS, proving the surface evilginx proxies to actually
# works. It runs on high ports with no root.
#
# The full evilginx proxy loop (lure host -> upstream) needs privileged ports or
# EVILGINX_HTTPS_PORT + setcap; see tests/smoke/aitm_e2e_sudo.sh for that path.
#
#   bash tests/smoke/demo_cookie_e2e.sh
# =============================================================================
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO"

HTTP_PORT="${HTTP_PORT:-19080}"
TLS_PORT="${TLS_PORT:-19443}"
CERT_DIR="$(mktemp -d)"
USER_EMAIL="demo@phishkit.local"
USER_PASS="demo-password"
PIDS=()

cleanup() {
  for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$CERT_DIR"
}
trap cleanup EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok   - $*"; }

wait_for_port() {
  local scheme="$1" host="$2" port="$3" i
  for i in $(seq 1 50); do
    if curl -sk -o /dev/null "$scheme://$host:$port/healthz" 2>/dev/null; then return 0; fi
    sleep 0.2
  done
  return 1
}

echo "==> deps"
(cd demos && npm install --silent >/dev/null 2>&1)

echo "==> self-signed cert for the HTTPS upstream (app.cookie-demo.test)"
scripts/gen_demo_tls_cert.sh "$CERT_DIR" app.cookie-demo.test >/dev/null
CERT="$CERT_DIR/app.cookie-demo.test.crt"
KEY="$CERT_DIR/app.cookie-demo.test.key"
[[ -f "$CERT" && -f "$KEY" ]] || fail "cert generation"
pass "cert generated"

echo "==> start cookie demo over HTTP (:$HTTP_PORT)"
( cd demos && HOST=127.0.0.1 PORT="$HTTP_PORT" npx tsx cookie/server.ts >/dev/null 2>&1 ) &
PIDS+=($!)
echo "==> start cookie demo over HTTPS upstream (:$TLS_PORT)"
( cd demos && HOST=127.0.0.1 PORT="$TLS_PORT" TLS_CERT="$CERT" TLS_KEY="$KEY" npx tsx cookie/server.ts >/dev/null 2>&1 ) &
PIDS+=($!)

wait_for_port http  127.0.0.1 "$HTTP_PORT" || fail "HTTP demo did not come up"
pass "HTTP demo up"
wait_for_port https 127.0.0.1 "$TLS_PORT"  || fail "HTTPS upstream did not come up"
pass "HTTPS upstream reachable (distinct from lure host)"

echo "==> login page served over HTTPS"
code=$(curl -sk -o /dev/null -w '%{http_code}' "https://127.0.0.1:$TLS_PORT/login")
[[ "$code" == "200" ]] || fail "login page HTTPS status $code"
pass "GET /login over HTTPS -> 200"

echo "==> bad credentials rejected"
code=$(curl -sk -o /dev/null -w '%{http_code}' -X POST \
  --data "username=$USER_EMAIL&password=wrong" "https://127.0.0.1:$TLS_PORT/login")
[[ "$code" == "401" ]] || fail "bad creds got $code (expected 401)"
pass "bad creds -> 401"

echo "==> valid login issues a session cookie"
JAR="$CERT_DIR/jar.txt"
loc=$(curl -sk -c "$JAR" -o /dev/null -w '%{redirect_url}\n' -X POST \
  --data "username=$USER_EMAIL&password=$USER_PASS" "https://127.0.0.1:$TLS_PORT/login")
grep -q "session" "$JAR" || fail "no session cookie set"
case "$loc" in */dashboard) ;; *) fail "login redirect was '$loc' (expected /dashboard)";; esac
pass "valid login -> Set-Cookie session, 302 /dashboard"

echo "==> authenticated dashboard reachable with the captured cookie"
body=$(curl -sk -b "$JAR" "https://127.0.0.1:$TLS_PORT/dashboard")
echo "$body" | grep -q "Signed in" || fail "dashboard not authenticated"
pass "GET /dashboard with session cookie -> Signed in"

echo
echo "PASS: cookie AiTM capture surface works end to end over a distinct HTTPS upstream."
