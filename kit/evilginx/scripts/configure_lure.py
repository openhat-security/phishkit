#!/usr/bin/env python3
"""
Configure evilginx for a dry-run engagement and print the lure URL.

Uses pexpect to drive the evilginx REPL (screen -X stuff does not work on macOS
because evilginx requires a real TTY). After config is persisted to config.json,
evilginx is restarted inside the phishkit screen session for background use.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import secrets
import string
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _repo_root() -> Path:
    """Resolve the phishkit checkout (not kit/). Prefer env from the Rust engine."""
    for key in ("PHISHKIT_ROOT", "KIT_ROOT"):
        raw = (os.environ.get(key) or "").strip()
        if raw:
            return Path(raw).resolve()
    # kit/evilginx/scripts → repo root is three parents
    return HERE.parent.parent.parent


KIT_ROOT = _repo_root()
EVIL_DIR = KIT_ROOT / "kit" / "evilginx"
EVILGINX_BIN = EVIL_DIR / "run" / "evilginx"
PHISHLETS_DIR = EVIL_DIR / "phishlets"
REDIRECTORS_DIR = KIT_ROOT / "vendor" / "evilginx2" / "redirectors"
# Prefer OS data dir from the desktop/CLI engine; fall back to kit-tree runtime.
_DATA_ENV = (os.environ.get("EVILGINX_DATA_DIR") or "").strip()
DATA_DIR = Path(_DATA_ENV).resolve() if _DATA_ENV else (EVIL_DIR / "run" / "data")
CONFIG_JSON = DATA_DIR / "config.json"
_LOG_ENV = (os.environ.get("EVILGINX_LOG") or "").strip()
EVILGINX_LOG = Path(_LOG_ENV).resolve() if _LOG_ENV else (DATA_DIR / "evilginx.log")
SCREEN_NAME = "phishkit-evilginx"

class ConfigureError(RuntimeError):
    pass


def _log(msg: str) -> None:
    print(f"[configure-lure] {msg}", file=sys.stderr)


def _screen_ids() -> list[str]:
    try:
        out = subprocess.run(
            ["screen", "-ls"],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return []
    ids: list[str] = []
    for line in (out.stdout or "").splitlines():
        if f".{SCREEN_NAME}" in line:
            parts = line.strip().split()
            if parts:
                ids.append(parts[0])
    return ids


def stop_evilginx() -> None:
    for sid in _screen_ids():
        _log(f"quitting screen session {sid}")
        subprocess.run(["screen", "-S", sid, "-X", "quit"], check=False)
        time.sleep(0.2)
    subprocess.run(["pkill", "-f", f"evilginx.*{DATA_DIR}"], check=False)
    time.sleep(0.5)


def _enabled_phishlets() -> list[str]:
    if not CONFIG_JSON.is_file():
        return []
    try:
        cfg = json.loads(CONFIG_JSON.read_text())
    except Exception:
        return []
    out: list[str] = []
    for name, meta in (cfg.get("phishlets") or {}).items():
        if meta.get("enabled"):
            out.append(name)
    return out


def _landing_sub(phishlet: str) -> str:
    """phish_sub of is_landing host. Empty string for apex sites (no portal. invent)."""
    path = PHISHLETS_DIR / f"{phishlet}.yaml"
    if not path.is_file():
        return ""
    for line in path.read_text(errors="replace").splitlines():
        if "is_landing: true" in line or "is_landing:true" in line:
            m = re.search(r"phish_sub:\s*'([^']*)'", line)
            if m:
                return m.group(1)
    return ""


def _login_path(phishlet: str) -> str:
    path = PHISHLETS_DIR / f"{phishlet}.yaml"
    if not path.is_file():
        return "/"
    text = path.read_text(errors="replace")
    m = re.search(r"login:\s*\n(?:[^\n]+\n)*?\s*path:\s*'([^']*)'", text)
    if m:
        p = m.group(1).strip()
        return p if p.startswith("/") else f"/{p}"
    return "/"


def _random_lure_path() -> str:
    alphabet = string.ascii_letters + string.digits
    token = "".join(secrets.choice(alphabet) for _ in range(8))
    return f"/{token}"


def _existing_lure_path(cfg: dict, phishlet: str, dryrun_domain: str) -> str | None:
    """Keep the current lure token when re-applying config (avoids stale bookmarked URLs)."""
    if (cfg.get("general") or {}).get("domain") != dryrun_domain:
        return None
    for lure in cfg.get("lures") or []:
        if lure.get("phishlet") == phishlet:
            path = (lure.get("path") or "").strip()
            if path:
                return path if path.startswith("/") else f"/{path}"
    return None


def _pick_lure_path(phishlet: str) -> str:
    """
    Evilginx redirects lure path -> phishlet login path only when they differ.
    If lure path equals login path, requests on the lure hostname 404.
    """
    login = _login_path(phishlet)
    if login == "/":
        # Apex / root-login sites: stable /login lure avoids root-path 404 quirks.
        return "/login"
    lure = _random_lure_path()
    while lure == login:
        lure = _random_lure_path()
    return lure


def lure_from_config(dryrun_domain: str, phishlet: str) -> str | None:
    if not CONFIG_JSON.is_file():
        return None
    try:
        cfg = json.loads(CONFIG_JSON.read_text())
    except Exception:
        return None
    domain = (cfg.get("general") or {}).get("domain") or ""
    lures = cfg.get("lures") or []
    if domain != dryrun_domain or not lures:
        return None
    if lures[0].get("phishlet") != phishlet:
        return None
    path = lures[0].get("path") or ""
    lure_host = (lures[0].get("hostname") or "").strip()
    if lure_host:
        host = lure_host
    else:
        sub = _landing_sub(phishlet)
        host = f"{sub}.{domain}" if sub else domain
    return f"https://{host}{path}"


def _normalize_lure_ops(ops: dict | None) -> dict:
    ops = ops or {}
    redirector = (ops.get("redirector") or "").strip()
    if redirector:
        # Allow bare directory name under vendor redirectors
        rpath = REDIRECTORS_DIR / redirector
        if not rpath.is_dir():
            raise ConfigureError(
                f"redirector '{redirector}' not found under {REDIRECTORS_DIR}"
            )
    path = (ops.get("path") or "").strip()
    if path and not path.startswith("/"):
        path = "/" + path
    extra = ops.get("extra_paths") or ops.get("extraPaths") or []
    extra_paths: list[str] = []
    for p in extra:
        p = str(p).strip()
        if not p:
            continue
        if not p.startswith("/"):
            p = "/" + p
        if p not in extra_paths and p != path:
            extra_paths.append(p)
    paused = ops.get("paused")
    paused_val = 0
    if paused is True or paused == 1 or paused == "1" or paused == "true":
        paused_val = int(time.time()) + 365 * 24 * 3600
    elif isinstance(paused, (int, float)) and paused > 0:
        paused_val = int(paused)
    return {
        "redirect_url": (ops.get("redirect_url") or ops.get("redirectUrl") or "").strip(),
        "redirector": redirector,
        "ua_filter": (ops.get("ua_filter") or ops.get("uaFilter") or "").strip(),
        "og_title": (ops.get("og_title") or ops.get("ogTitle") or "").strip(),
        "og_desc": (ops.get("og_desc") or ops.get("ogDesc") or "").strip(),
        "og_image": (ops.get("og_image") or ops.get("ogImage") or "").strip(),
        "og_url": (ops.get("og_url") or ops.get("ogUrl") or "").strip(),
        "path": path,
        "extra_paths": extra_paths,
        "paused": paused_val,
        "regenerate_path": bool(
            ops.get("regenerate_path") or ops.get("regeneratePath")
        ),
    }


def configure_via_config_json(
    phishlet: str,
    dryrun_domain: str,
    profile_id: str | None = None,
    lure_ops: dict | None = None,
    lures_list: list[dict] | None = None,
) -> str:
    if not EVILGINX_BIN.is_file():
        raise ConfigureError(
            f"evilginx binary missing at {EVILGINX_BIN} — run make build-evilginx"
        )
    phishlet_yaml = PHISHLETS_DIR / f"{phishlet}.yaml"
    if not phishlet_yaml.is_file():
        raise ConfigureError(f"phishlet not found: {phishlet_yaml}")

    DATA_DIR.mkdir(parents=True, exist_ok=True)
    EVILGINX_LOG.parent.mkdir(parents=True, exist_ok=True)

    cfg: dict = {}
    if CONFIG_JSON.is_file():
        try:
            cfg = json.loads(CONFIG_JSON.read_text())
        except Exception:
            cfg = {}

    general = cfg.get("general") or {}
    general["domain"] = dryrun_domain
    general["external_ipv4"] = "127.0.0.1"
    # Empty = 403 on unauthorized requests (Evilginx default is a Rick Roll URL).
    general["unauth_url"] = ""
    cfg["general"] = general

    # Local dry-run: default "unauth" blacklists 127.0.0.1 after failed probes and
    # then 403s static assets / API paths that aren't lure hits.
    cfg["blacklist"] = {"mode": "off"}
    bl_path = DATA_DIR / "blacklist.txt"
    if bl_path.is_file() and bl_path.stat().st_size > 0:
        bl_path.write_text("")
        _log("cleared blacklist.txt")

    ph_cfg = cfg.get("phishlets") or {}
    for name in list(ph_cfg.keys()):
        meta = ph_cfg.get(name) or {}
        meta["enabled"] = False
        ph_cfg[name] = meta

    target_meta = ph_cfg.get(phishlet) or {}
    target_meta["enabled"] = True
    target_meta["visible"] = True
    target_meta["hostname"] = dryrun_domain
    if "unauth_url" not in target_meta:
        target_meta["unauth_url"] = ""
    ph_cfg[phishlet] = target_meta
    cfg["phishlets"] = ph_cfg

    landing_sub = _landing_sub(phishlet)
    landing_host = f"{landing_sub}.{dryrun_domain}" if landing_sub else dryrun_domain
    lure_info = ""
    if profile_id:
        lure_info = f"profile:{profile_id.strip()}"

    def _lure_entry(path: str, ops: dict, info: str = lure_info) -> dict:
        return {
            "hostname": "",
            "path": path,
            "redirect_url": ops["redirect_url"],
            "phishlet": phishlet,
            "redirector": ops["redirector"],
            "ua_filter": ops["ua_filter"],
            "info": info,
            "og_title": ops["og_title"],
            "og_desc": ops["og_desc"],
            "og_image": ops["og_image"],
            "og_url": ops["og_url"],
            "paused": ops["paused"],
        }

    def _resolve_lure_path(ops: dict) -> str:
        if ops["regenerate_path"] or not ops["path"]:
            return (
                _pick_lure_path(phishlet)
                if ops["regenerate_path"]
                else (
                    _existing_lure_path(cfg, phishlet, dryrun_domain)
                    or _pick_lure_path(phishlet)
                )
            )
        return ops["path"]

    if lures_list:
        lures = []
        for i, item in enumerate(lures_list):
            if not isinstance(item, dict):
                raise ConfigureError("--lures-json entries must be objects")
            ops = _normalize_lure_ops(item)
            lure_path = _resolve_lure_path(ops)
            info = f"{lure_info}:{i}" if lure_info else f"lure:{i}"
            lures.append(_lure_entry(lure_path, ops, info=info))
            _log(f"multi-lure path={lure_path}")
        if not lures:
            raise ConfigureError("--lures-json must contain at least one lure object")
        cfg["lures"] = lures
    else:
        ops = _normalize_lure_ops(lure_ops)
        lure_path = _resolve_lure_path(ops)
        _log(f"lure path={lure_path} on {landing_host} (login path={_login_path(phishlet)})")

        lures = [_lure_entry(lure_path, ops)]
        for i, ep in enumerate(ops["extra_paths"]):
            info = f"{lure_info}:extra:{i}" if lure_info else f"extra:{i}"
            lures.append(_lure_entry(ep, ops, info=info))
            _log(f"extra lure path={ep}")
        if ops["redirect_url"]:
            _log(f"post-capture redirect → {ops['redirect_url']}")
        if ops["redirector"]:
            _log(f"html redirector={ops['redirector']}")
        if ops["ua_filter"]:
            _log(f"ua_filter={ops['ua_filter']}")
        if ops["og_title"] or ops["og_desc"] or ops["og_image"]:
            _log("og tags set for link previews")
        cfg["lures"] = lures

    CONFIG_JSON.write_text(json.dumps(cfg, indent=2) + "\n")
    first_path = (cfg["lures"][0].get("path") or "").strip()
    if not first_path.startswith("/"):
        first_path = f"/{first_path}" if first_path else _pick_lure_path(phishlet)
    return lure_from_config(dryrun_domain, phishlet) or f"https://{landing_host}{first_path}"

def _https_port_listening() -> bool:
    probe = subprocess.run(
        ["lsof", "-nP", "-iTCP:443", "-sTCP:LISTEN", "-t"],
        capture_output=True,
        text=True,
        check=False,
    )
    return bool(probe.stdout.strip())


def _wait_https_port(timeout_sec: float = 12.0) -> None:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        if _https_port_listening():
            return
        time.sleep(0.25)
    raise ConfigureError(
        "evilginx did not bind HTTPS port 443 in time — "
        "check evilginx.log (port in use or need elevated privileges)."
    )


def start_in_screen() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    EVILGINX_LOG.parent.mkdir(parents=True, exist_ok=True)
    env_export = f"export PHISHKIT_ROOT={KIT_ROOT}; "
    cmd = (
        f"{env_export}exec {EVILGINX_BIN} -p {PHISHLETS_DIR} -t {REDIRECTORS_DIR} -c {DATA_DIR}"
        f" -developer -debug 2>&1 | tee -a {EVILGINX_LOG}"
    )
    args = ["screen", "-dmS", SCREEN_NAME, "bash", "-lc", cmd]
    _log(f"starting background screen session '{SCREEN_NAME}'")
    subprocess.run(args, check=False, cwd=str(KIT_ROOT))

    for _ in range(40):
        probe = subprocess.run(
            ["pgrep", "-f", f"evilginx.*{DATA_DIR}"],
            capture_output=True,
            check=False,
        )
        if probe.returncode == 0:
            _wait_https_port()
            _log("evilginx is running in screen (port 443 listening)")
            return
        time.sleep(0.25)
    raise ConfigureError("evilginx did not start in screen after configure")


def _probe_lure(lure_url: str) -> bool:
    import ssl
    import urllib.error
    import urllib.request

    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    # Do not follow redirects: lure returns 302 → /login without a cookie jar,
    # and the follow-up GET looks "unauthorized" to evilginx (false negative).
    class _NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
            return None

    opener = urllib.request.build_opener(
        urllib.request.HTTPSHandler(context=ctx),
        _NoRedirect(),
    )
    req = urllib.request.Request(
        lure_url,
        headers={
            "User-Agent": "phishkit-probe/1.0",
            "X-Phishkit-Healthcheck": "1",
        },
    )
    try:
        with opener.open(req, timeout=10) as resp:
            return resp.getcode() < 400
    except urllib.error.HTTPError as e:
        # 3xx without Location follow still surfaces as HTTPError for some handlers
        return e.code < 400 or e.code in (301, 302, 303, 307, 308)
    except Exception:
        return False


def configure_and_get_lure(
    phishlet: str,
    dryrun_domain: str,
    profile_id: str | None = None,
    lure_ops: dict | None = None,
    lures_list: list[dict] | None = None,
) -> str:
    stop_evilginx()
    lure = configure_via_config_json(
        phishlet,
        dryrun_domain,
        profile_id=profile_id,
        lure_ops=lure_ops,
        lures_list=lures_list,
    )
    start_in_screen()
    persisted = lure_from_config(dryrun_domain, phishlet) or lure
    if not _probe_lure(persisted):
        raise ConfigureError(
            f"lure URL did not respond OK after restart: {persisted}. "
            "Check /etc/hosts for all phishlet subdomains and click Start + get lure again."
        )
    _log(f"lure ready: {persisted}")
    return persisted


def main() -> int:
    parser = argparse.ArgumentParser(description="Configure evilginx and print lure URL")
    parser.add_argument("--phishlet", required=True)
    parser.add_argument("--dryrun-domain", required=True)
    parser.add_argument("--profile-id", default="")
    parser.add_argument(
        "--lure-ops-json",
        default="",
        help="JSON object: redirect_url, og_*, ua_filter, redirector, path, extra_paths, paused",
    )
    parser.add_argument(
        "--lures-json",
        default="",
        help="JSON array of lure ops objects (multi-lure configure)",
    )
    parser.add_argument("--redirect-url", default="")
    parser.add_argument("--og-title", default="")
    parser.add_argument("--og-desc", default="")
    parser.add_argument("--og-image", default="")
    parser.add_argument("--og-url", default="")
    parser.add_argument("--ua-filter", default="")
    parser.add_argument("--redirector", default="")
    parser.add_argument("--lure-path", default="")
    parser.add_argument("--paused", action="store_true")
    parser.add_argument("--regenerate-path", action="store_true")
    args = parser.parse_args()
    try:
        pid = args.profile_id.strip() or None
        ops: dict = {}
        lures_list: list[dict] | None = None
        if args.lures_json.strip():
            try:
                parsed = json.loads(args.lures_json)
            except json.JSONDecodeError as e:
                raise ConfigureError(f"invalid --lures-json: {e}") from e
            if not isinstance(parsed, list):
                raise ConfigureError("--lures-json must be a JSON array")
            lures_list = parsed
        elif args.lure_ops_json.strip():
            try:
                ops = json.loads(args.lure_ops_json)
            except json.JSONDecodeError as e:
                raise ConfigureError(f"invalid --lure-ops-json: {e}") from e
            if not isinstance(ops, dict):
                raise ConfigureError("--lure-ops-json must be a JSON object")
        # Explicit CLI flags override JSON (single-lure mode only)
        if lures_list is None:
            for key, val in (
                ("redirect_url", args.redirect_url),
                ("og_title", args.og_title),
                ("og_desc", args.og_desc),
                ("og_image", args.og_image),
                ("og_url", args.og_url),
                ("ua_filter", args.ua_filter),
                ("redirector", args.redirector),
                ("path", args.lure_path),
            ):
                if val.strip():
                    ops[key] = val.strip()
            if args.paused:
                ops["paused"] = True
            if args.regenerate_path:
                ops["regenerate_path"] = True
        lure = configure_and_get_lure(
            args.phishlet,
            args.dryrun_domain,
            profile_id=pid,
            lure_ops=ops or None,
            lures_list=lures_list,
        )
    except ConfigureError as e:
        print(str(e), file=sys.stderr)
        return 1
    print(lure)
    return 0


if __name__ == "__main__":
    sys.exit(main())
