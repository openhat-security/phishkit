#!/usr/bin/env bash
# Generate a self-signed TLS cert for the local AiTM demo HTTPS upstream.
# evilginx proxies to this upstream with verification disabled (goproxy default),
# so a self-signed cert is fine for the dry-run.
#
# Usage: scripts/gen_demo_tls_cert.sh <out_dir> [cn]
set -euo pipefail

OUT_DIR="${1:-demos/.certs}"
CN="${2:-app.cookie-demo.test}"
mkdir -p "$OUT_DIR"
CERT="$OUT_DIR/$CN.crt"
KEY="$OUT_DIR/$CN.key"

if [[ -f "$CERT" && -f "$KEY" ]]; then
  echo "cert exists: $CERT"
  exit 0
fi

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$KEY" -out "$CERT" -days 365 \
  -subj "/CN=$CN" \
  -addext "subjectAltName=DNS:$CN,DNS:cookie-demo.test" >/dev/null 2>&1

echo "wrote $CERT and $KEY (CN=$CN)"
