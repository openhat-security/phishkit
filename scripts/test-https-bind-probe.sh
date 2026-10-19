#!/usr/bin/env bash
# Smoke/unit tests for the evilginx HTTPS bind probe (C1 false-negative fix).
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
import importlib.util
import socket
import threading
import time
import traceback
from pathlib import Path

SCRIPT = Path("kit/evilginx/scripts/configure_lure.py")
spec = importlib.util.spec_from_file_location("configure_lure", SCRIPT)
mod = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(mod)

failures = 0

def check(name, fn):
    global failures
    try:
        fn()
        print(f"ok   {name}")
    except Exception as e:
        failures += 1
        print(f"FAIL {name}: {e}")
        traceback.print_exc()

def test_false_when_free():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    time.sleep(0.05)
    assert mod._https_port_listening(port) is False

def test_true_when_listening():
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)
    port = srv.getsockname()[1]
    def _accept():
        try:
            c, _ = srv.accept()
            c.close()
        except OSError:
            pass
    t = threading.Thread(target=_accept, daemon=True)
    t.start()
    try:
        assert mod._https_port_listening(port) is True
    finally:
        srv.close()
        t.join(timeout=1)

def test_late_bind():
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", 0))
    port = srv.getsockname()[1]
    def _late():
        time.sleep(0.4)
        try:
            srv.listen(1)
            c, _ = srv.accept()
            c.close()
        except OSError:
            pass
    t = threading.Thread(target=_late, daemon=True)
    t.start()
    ok = False
    deadline = time.time() + 3
    while time.time() < deadline:
        if mod._https_port_listening(port):
            ok = True
            break
        time.sleep(0.1)
    srv.close()
    t.join(timeout=1)
    assert ok

check("false_when_free", test_false_when_free)
check("true_when_listening", test_true_when_listening)
check("late_bind", test_late_bind)
raise SystemExit(failures)
PY
