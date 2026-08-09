#!/usr/bin/env python3
"""Record Playwright videos of localhost demo login flows for VitePress docs.

Starts neither demo servers nor evilginx — callers must already have demos up
(or use `make docs-videos`). Built-in demo credentials; no mailbox required.

  make docs-videos
  # or: python3 scripts/e2e_demo_videos.py

Outputs normalized MP4s under docs/media/.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

KIT_ROOT = Path(__file__).resolve().parent.parent
OUT = KIT_ROOT / "run" / "e2e-videos"
RAW = OUT / "_raw"


def log(msg: str) -> None:
    print(f"[demo-videos] {msg}", flush=True)


def wait_http(url: str, timeout: float = 30.0) -> None:
    deadline = time.time() + timeout
    last = ""
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=2) as resp:
                if resp.status < 500:
                    return
        except Exception as e:  # noqa: BLE001 — probe loop
            last = str(e)
        time.sleep(0.4)
    raise RuntimeError(f"timeout waiting for {url}: {last}")


def record_login(name: str, base: str, email: str, password: str) -> Path:
    from playwright.sync_api import sync_playwright

    RAW.mkdir(parents=True, exist_ok=True)
    dest_dir = RAW / name
    if dest_dir.exists():
        shutil.rmtree(dest_dir)
    dest_dir.mkdir(parents=True)

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(record_video_dir=str(dest_dir), record_video_size={"width": 1280, "height": 720})
        page = context.new_page()
        page.goto(base, wait_until="domcontentloaded")
        page.wait_for_timeout(800)

        # Portal: form on /login ; Firebase: similar form on /
        if "9080" in base:
            page.goto(f"{base.rstrip('/')}/login", wait_until="domcontentloaded")
        page.wait_for_timeout(500)

        email_sel = 'input[type="email"], input[name="email"], input[name="username"], #email'
        pass_sel = 'input[type="password"], input[name="password"], #password'
        page.fill(email_sel, email)
        page.fill(pass_sel, password)
        # Portal uses form submit; Firebase mock uses #signin button.
        if page.query_selector("#signin"):
            page.click("#signin")
        else:
            page.click('button[type="submit"], button:has-text("Sign in"), button:has-text("Log in")')
        page.wait_for_timeout(2500)
        context.close()
        browser.close()

    webs = list(dest_dir.glob("*.webm"))
    if not webs:
        raise RuntimeError(f"no webm recorded for {name}")
    return webs[0]


def to_mp4(webm: Path, out_mp4: Path) -> None:
    out_mp4.parent.mkdir(parents=True, exist_ok=True)
    if shutil.which("ffmpeg"):
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-i",
                str(webm),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-an",
                str(out_mp4),
            ],
            check=True,
            capture_output=True,
        )
    else:
        # Fallback: copy webm with .mp4 name so docs pipeline still has an artifact;
        # prefer installing ffmpeg for real MP4s.
        shutil.copy2(webm, out_mp4.with_suffix(".webm"))
        log(f"ffmpeg missing — left {out_mp4.with_suffix('.webm')} (install ffmpeg for mp4)")
        return
    log(f"wrote {out_mp4}")


def main() -> int:
    email = "demo@phishkit.local"
    password = "demo-password"
    OUT.mkdir(parents=True, exist_ok=True)

    try:
        wait_http("http://127.0.0.1:9080/")
        wait_http("http://127.0.0.1:9081/")
    except RuntimeError as e:
        print(f"[demo-videos] FAIL: {e}\nStart demos with: make demo", file=sys.stderr)
        return 1

    try:
        from playwright.sync_api import sync_playwright  # noqa: F401
    except ImportError:
        print(
            "[demo-videos] FAIL: playwright not installed.\n"
            "  pip install playwright && python3 -m playwright install chromium",
            file=sys.stderr,
        )
        return 1

    portal_webm = record_login("portal", "http://127.0.0.1:9080", email, password)
    to_mp4(portal_webm, OUT / "walkthrough-demo-login.mp4")

    firebase_webm = record_login("firebase", "http://127.0.0.1:9081", email, password)
    to_mp4(firebase_webm, OUT / "walkthrough-demo-firebase.mp4")

    log("PASS — demo videos recorded")
    return 0


if __name__ == "__main__":
    # Fix typo in type hint above if any — Python 3.11+
    raise SystemExit(main())
