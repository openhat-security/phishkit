#!/usr/bin/env python3
"""
Open the real target in an isolated browser window and run a Firebase inject script.

Used by the web UI "Launch incognito" action on captured sessions.
"""
from __future__ import annotations

import argparse
import platform
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def _launch_playwright(url: str, script: str) -> None:
    from playwright.sync_api import sync_playwright

    with sync_playwright() as p:
        launch_args = ["--incognito"] if platform.system() == "Darwin" else []
        try:
            browser = p.chromium.launch(
                channel="chrome",
                headless=False,
                args=launch_args,
            )
        except Exception:
            browser = p.chromium.launch(headless=False, args=launch_args)

        context = browser.new_context()
        page = context.new_page()
        page.goto(url, wait_until="domcontentloaded", timeout=90_000)
        page.evaluate(script)
        print(f"[launch-inject] ran inject on {url}", flush=True)
        print("[launch-inject] close the browser window when finished", flush=True)

        try:
            while browser.contexts:
                pages = browser.contexts[0].pages
                if not pages:
                    break
                time.sleep(0.5)
        except KeyboardInterrupt:
            pass
        finally:
            browser.close()


def _fallback_macos(url: str, script: str) -> int:
    """Open Chrome incognito and copy script — user pastes in DevTools if Playwright missing."""
    subprocess.run(["pbcopy"], input=script.encode(), check=False)
    subprocess.run(
        ["open", "-na", "Google Chrome", "--args", "--incognito", url],
        check=False,
    )
    print(
        "[launch-inject] Playwright not available — opened Chrome incognito and copied "
        "the inject script to your clipboard. Paste it in DevTools → Console.",
        file=sys.stderr,
        flush=True,
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Launch incognito browser and inject session")
    parser.add_argument("--url", required=True, help="Real target URL (https://app.client.com/...)")
    parser.add_argument("--script-file", required=True, help="Path to JS inject snippet")
    args = parser.parse_args()

    script_path = Path(args.script_file)
    if not script_path.is_file():
        print(f"script file not found: {script_path}", file=sys.stderr)
        return 1

    script = script_path.read_text(encoding="utf-8")
    url = args.url.strip()

    try:
        _launch_playwright(url, script)
        return 0
    except ImportError:
        if platform.system() == "Darwin":
            return _fallback_macos(url, script)
        print(
            "Install Playwright: pip install playwright && playwright install chrome",
            file=sys.stderr,
        )
        return 1
    except Exception as e:  # noqa: BLE001
        print(f"launch failed: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
