#!/usr/bin/env bash
# Build the Linux desktop binary with test hooks and run the WDIO suite on Xvfb.
set -euo pipefail

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"
ROOT="${PHISHKIT_ROOT:-/src}"
cd "$ROOT"

export PHISHKIT_CONFIG="${PHISHKIT_CONFIG:-/tmp/phishkit-test/config}"
export PHISHKIT_DATA="${PHISHKIT_DATA:-/tmp/phishkit-test/data}"
mkdir -p "$PHISHKIT_CONFIG" "$PHISHKIT_DATA" tests/integration/artifacts

echo "==> cookie-session demo on :9080"
(cd demos && npm install --silent && npm run demo:cookie) &
DEMO_PID=$!
trap 'kill "$DEMO_PID" 2>/dev/null || true' EXIT
sleep 2

echo "==> frontend (VITE_TEST_HOOKS=1)"
(cd apps/desktop && npm install && VITE_TEST_HOOKS=1 npm run build)

echo "==> desktop binary (--features test-hooks,custom-protocol)"
cargo build --release -p phishkit --features test-hooks,custom-protocol
export PHISHKIT_TEST_BIN="${PHISHKIT_TEST_BIN:-$ROOT/target/release/phishkit}"

echo "==> WebdriverIO on Xvfb"
(cd tests/integration && npm install)
if command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a -s "-screen 0 1280x800x24" npm --prefix tests/integration test
else
  npm --prefix tests/integration test
fi
