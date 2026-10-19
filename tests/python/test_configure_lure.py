#!/usr/bin/env python3
"""Unit tests for kit/evilginx/scripts/configure_lure.py.

Covers the parsing, port-selection, and bind-poll logic that the audit (C1.1–C1.3)
flagged. Pure stdlib (unittest) so it runs without extra deps:

    python3 -m unittest discover -s tests/python -v
"""
from __future__ import annotations

import importlib.util
import json
import os
import socket
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "kit" / "evilginx" / "scripts" / "configure_lure.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("configure_lure", SCRIPT)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(mod)
    return mod


cl = _load_module()


class PortSelection(unittest.TestCase):
    def setUp(self):
        for k in ("EVILGINX_HTTPS_PORT", "EVILGINX_DNS_PORT"):
            os.environ.pop(k, None)

    def tearDown(self):
        for k in ("EVILGINX_HTTPS_PORT", "EVILGINX_DNS_PORT"):
            os.environ.pop(k, None)

    def test_defaults_are_privileged_ports(self):
        self.assertEqual(cl.desired_https_port(), 443)
        self.assertEqual(cl.desired_dns_port(), 53)

    def test_env_override_high_ports(self):
        os.environ["EVILGINX_HTTPS_PORT"] = "8443"
        os.environ["EVILGINX_DNS_PORT"] = "5353"
        self.assertEqual(cl.desired_https_port(), 8443)
        self.assertEqual(cl.desired_dns_port(), 5353)

    def test_invalid_port_raises(self):
        os.environ["EVILGINX_HTTPS_PORT"] = "notaport"
        with self.assertRaises(cl.ConfigureError):
            cl.desired_https_port()

    def test_out_of_range_raises(self):
        os.environ["EVILGINX_HTTPS_PORT"] = "70000"
        with self.assertRaises(cl.ConfigureError):
            cl.desired_https_port()


class SocketProbe(unittest.TestCase):
    def test_socket_listening_true_when_bound(self):
        srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        srv.bind(("127.0.0.1", 0))
        srv.listen(1)
        port = srv.getsockname()[1]
        try:
            self.assertTrue(cl._socket_listening(port))
        finally:
            srv.close()

    def test_socket_listening_false_when_free(self):
        # Grab then release a port so it is almost certainly free.
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
        s.close()
        self.assertFalse(cl._socket_listening(port))


class WaitPoll(unittest.TestCase):
    def test_wait_returns_true_quickly_when_listening(self):
        orig = cl.https_port_listening
        cl.https_port_listening = lambda port=None: True
        try:
            self.assertTrue(cl._wait_https_port(8443, timeout_sec=1.0))
        finally:
            cl.https_port_listening = orig

    def test_wait_returns_false_without_raising_on_timeout(self):
        orig = cl.https_port_listening
        cl.https_port_listening = lambda port=None: False
        try:
            # Must NOT raise — caller decides what to do (re-check / teardown).
            self.assertFalse(cl._wait_https_port(8443, timeout_sec=0.3))
        finally:
            cl.https_port_listening = orig


class LurePaths(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self._saved = cl.PHISHLETS_DIR
        cl.PHISHLETS_DIR = self.dir

    def tearDown(self):
        cl.PHISHLETS_DIR = self._saved
        self.tmp.cleanup()

    def _write_phishlet(self, name: str, login_path: str = "/login"):
        (self.dir / f"{name}.yaml").write_text(
            "proxy_hosts:\n"
            "  - {phish_sub: '', orig_sub: 'app', domain: 'cookie-demo.test', is_landing: true}\n"
            "login:\n"
            f"  domain: 'app.cookie-demo.test'\n"
            f"  path: '{login_path}'\n"
        )

    def test_pick_lure_path_differs_from_login(self):
        self._write_phishlet("demo", login_path="/signin")
        path = cl._pick_lure_path("demo")
        self.assertTrue(path.startswith("/"))
        self.assertNotEqual(path, "/signin")

    def test_pick_lure_path_root_login_uses_login_alias(self):
        self._write_phishlet("demo", login_path="/")
        self.assertEqual(cl._pick_lure_path("demo"), "/login")

    def test_login_path_parsed(self):
        self._write_phishlet("demo", login_path="/account/login")
        self.assertEqual(cl._login_path("demo"), "/account/login")


class NormalizeOps(unittest.TestCase):
    def test_path_is_slash_prefixed(self):
        ops = cl._normalize_lure_ops({"path": "promo"})
        self.assertEqual(ops["path"], "/promo")

    def test_extra_paths_dedup_and_prefix(self):
        ops = cl._normalize_lure_ops({"path": "/a", "extra_paths": ["a", "b", "/b", "c"]})
        self.assertEqual(ops["extra_paths"], ["/b", "/c"])

    def test_empty_ops_ok(self):
        ops = cl._normalize_lure_ops(None)
        self.assertEqual(ops["path"], "")
        self.assertEqual(ops["extra_paths"], [])


class ConfigWritesPorts(unittest.TestCase):
    """configure_via_config_json must persist the chosen https/dns ports and enable
    only the target phishlet (so no colliding duplicate stays enabled)."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.saved = {
            "EVILGINX_BIN": cl.EVILGINX_BIN,
            "PHISHLETS_DIR": cl.PHISHLETS_DIR,
            "DATA_DIR": cl.DATA_DIR,
            "CONFIG_JSON": cl.CONFIG_JSON,
            "EVILGINX_LOG": cl.EVILGINX_LOG,
        }
        bin_path = root / "evilginx"
        bin_path.write_text("#!/bin/sh\n")
        phishlets = root / "phishlets"
        phishlets.mkdir()
        (phishlets / "demo-cookie.yaml").write_text(
            "proxy_hosts:\n"
            "  - {phish_sub: '', orig_sub: 'app', domain: 'cookie-demo.test', is_landing: true}\n"
            "login:\n  domain: 'app.cookie-demo.test'\n  path: '/login'\n"
        )
        (phishlets / "stale.yaml").write_text(
            "proxy_hosts:\n  - {phish_sub: '', domain: 'stale.test', is_landing: true}\n"
        )
        data = root / "data"
        data.mkdir()
        cl.EVILGINX_BIN = bin_path
        cl.PHISHLETS_DIR = phishlets
        cl.DATA_DIR = data
        cl.CONFIG_JSON = data / "config.json"
        cl.EVILGINX_LOG = data / "evilginx.log"
        # Pre-seed a config with a stale enabled phishlet to prove it gets disabled.
        cl.CONFIG_JSON.write_text(json.dumps({"phishlets": {"stale": {"enabled": True}}}))

    def tearDown(self):
        for k, v in self.saved.items():
            setattr(cl, k, v)
        for k in ("EVILGINX_HTTPS_PORT", "EVILGINX_DNS_PORT"):
            os.environ.pop(k, None)
        self.tmp.cleanup()

    def test_writes_high_ports_and_enables_only_target(self):
        os.environ["EVILGINX_HTTPS_PORT"] = "8443"
        os.environ["EVILGINX_DNS_PORT"] = "5353"
        lure = cl.configure_via_config_json("demo-cookie", "demo-cookie.local.phishkit")
        self.assertTrue(lure.startswith("https://demo-cookie.local.phishkit"))
        cfg = json.loads(cl.CONFIG_JSON.read_text())
        self.assertEqual(cfg["general"]["https_port"], 8443)
        self.assertEqual(cfg["general"]["dns_port"], 5353)
        self.assertEqual(cfg["general"]["domain"], "demo-cookie.local.phishkit")
        self.assertTrue(cfg["phishlets"]["demo-cookie"]["enabled"])
        self.assertFalse(cfg["phishlets"]["stale"]["enabled"])
        self.assertEqual(cfg["lures"][0]["phishlet"], "demo-cookie")

    def test_defaults_to_privileged_ports(self):
        cl.configure_via_config_json("demo-cookie", "demo-cookie.local.phishkit")
        cfg = json.loads(cl.CONFIG_JSON.read_text())
        self.assertEqual(cfg["general"]["https_port"], 443)
        self.assertEqual(cfg["general"]["dns_port"], 53)


if __name__ == "__main__":
    unittest.main(verbosity=2)
