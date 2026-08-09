#!/usr/bin/env bash
# Shared env + helpers for the phishkit evilginx2 harness.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVIL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# kit/evilginx → repo root is two levels up
KIT_ROOT="$(cd "$EVIL_DIR/../.." && pwd)"

export KIT_ROOT
export EVIL_DIR

# Upstream evilginx2 source (git submodule at vendor/evilginx2).
export EVILGINX_SRC="${EVILGINX_SRC:-$KIT_ROOT/vendor/evilginx2}"

export RUN_DIR="${RUN_DIR:-$EVIL_DIR/run}"
export EVILGINX_BIN="$RUN_DIR/evilginx"
# Desktop/CLI set EVILGINX_DATA_DIR to the OS data dir; shell-only defaults stay under kit/.
export EVILGINX_DATA_DIR="${EVILGINX_DATA_DIR:-$RUN_DIR/data}"
export EVILGINX_LOG="${EVILGINX_LOG:-$EVILGINX_DATA_DIR/evilginx.log}"
export EVILGINX_PID="${EVILGINX_PID:-$EVILGINX_DATA_DIR/evilginx.pid}"

# Where we keep our (per-engagement) phishlets
export PHISHLETS_DIR="$EVIL_DIR/phishlets"
export REDIRECTORS_DIR="$EVILGINX_SRC/redirectors"

mkdir -p "$RUN_DIR" "$EVILGINX_DATA_DIR"

# Sanity check: make sure we are inside a real phishkit tree
if [[ ! -d "$KIT_ROOT/kit/evilginx/phishlets" || ! -f "$KIT_ROOT/Makefile" ]]; then
    echo "ERROR: Path calculation failed. KIT_ROOT resolved to '$KIT_ROOT'." >&2
    echo "This usually means the phishkit scripts were moved or you are running them from an unexpected location." >&2
    echo "Please run commands from inside the phishkit/ directory." >&2
    exit 1
fi

log()  { printf '\033[1;36m[%s]\033[0m %s\n' "$(basename "${BASH_SOURCE[1]:-?}")" "$*"; }
die()  { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }
warn() { printf '\033[1;33m[warn]\033[0m  %s\n' "$*"; }

check_port_free() {
    local port="$1"
    local pids
    pids="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null || true)"
    if [[ -z "$pids" ]]; then return 0; fi
    while IFS= read -r pid; do
        [[ -z "$pid" ]] && continue
        local cmd
        cmd="$(ps -p "$pid" -o comm= 2>/dev/null || echo '?')"
        die "port $port is already bound by pid $pid ($cmd)."
    done <<< "$pids"
}

wait_for_port() {
    local host="$1" port="$2" timeout="${3:-30}"
    local start=$SECONDS
    while ! (echo >"/dev/tcp/$host/$port") 2>/dev/null; do
        if (( SECONDS - start > timeout )); then
            die "timeout waiting for $host:$port"
        fi
        sleep 0.25
    done
}
