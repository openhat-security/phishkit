"""
Automation helpers for driving evilginx2 from Python (using pexpect).

This is the secret sauce that lets us get a real lure URL programmatically
instead of making the engineer copy-paste at the ':' prompt.

Only used by the "quick assessment" / fully automated paths.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional

try:
    import pexpect
except ImportError:
    pexpect = None  # type: ignore

HERE = Path(__file__).resolve().parent
KIT_ROOT = HERE.parent.parent


class EvilginxAutomationError(RuntimeError):
    pass


def _safe_text(v) -> str:
    if v is None:
        return ""
    if isinstance(v, str):
        return v
    if isinstance(v, bytes):
        return v.decode("utf-8", errors="replace")
    return str(v)


def _check_required_ports() -> None:
    # evilginx needs these privileged ports in dry-run mode too.
    for port in (443, 80, 53):
        try:
            pid_out = subprocess.run(
                ["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"],
                capture_output=True,
                text=True,
                check=False,
            )
        except Exception:
            continue

        pid = (pid_out.stdout or "").strip().splitlines()
        if not pid:
            continue
        first_pid = pid[0].strip()
        cmd = ""
        try:
            ps_out = subprocess.run(
                ["ps", "-p", first_pid, "-o", "comm="],
                capture_output=True,
                text=True,
                check=False,
            )
            cmd = (ps_out.stdout or "").strip()
        except Exception:
            pass

        raise EvilginxAutomationError(
            f"Required port {port} is already in use by pid {first_pid}"
            f"{f' ({cmd})' if cmd else ''}.\n"
            "Stop the conflicting process (or change your test setup) and retry."
        )


def _require_pexpect():
    if pexpect is None:
        raise EvilginxAutomationError(
            "The 'pexpect' package is required for fully automated evilginx control.\n"
            "Install it with:\n"
            "    pip install -r scripts/requirements.txt\n"
            "or\n"
            "    pip install pexpect"
        )


def get_lure_url_headless(
    phishlet_name: str,
    dryrun_domain: str = "example-phish.local.test",
    evilginx_bin: Optional[str] = None,
    timeout: float = 45.0,
) -> str:
    """
    Launch evilginx in developer (dry-run) mode, feed it the minimal config
    commands via the REPL, create a lure, and return the lure URL.

    This is the function the "one-command" engineer experience calls.

    The caller is responsible for ensuring:
      - The phishlet exists and validates.
      - Ports 53/80/443 are free or the user knows they may need sudo.
      - /etc/hosts will be updated for the dryrun_domain (we can do this too).
    """
    _require_pexpect()
    _check_required_ports()

    bin_path = evilginx_bin or str(KIT_ROOT / "evilginx" / "run" / "evilginx")
    if not os.path.isfile(bin_path) or not os.access(bin_path, os.X_OK):
        raise EvilginxAutomationError(
            f"evilginx binary not found or not executable at {bin_path}. "
            "Run 'make build-evilginx' first."
        )

    phishlets_dir = str(KIT_ROOT / "evilginx" / "phishlets")
    redirectors_dir = str(KIT_ROOT / "evilginx" / "run" / "redirectors")  # may not exist, evilginx falls back

    # We use the headless starter which avoids the interactive "press enter" prompt.
    launcher = HERE / "start_dryrun_headless.sh"
    if not launcher.exists():
        raise EvilginxAutomationError(
            f"Headless launcher not found: {launcher}\n"
            "Rebuild or restore phishkit files."
        )
    if not os.access(str(launcher), os.X_OK):
        # Auto-heal executable bit so users don't get blocked by file mode drift.
        try:
            launcher.chmod(0o755)
        except Exception:
            pass
    if not os.access(str(launcher), os.X_OK):
        raise EvilginxAutomationError(
            f"Headless launcher exists but is not executable: {launcher}\n"
            "Run: chmod +x evilginx/scripts/start_dryrun_headless.sh"
        )

    cmd = [str(launcher)]

    # The start script respects these env vars.
    env = os.environ.copy()
    env["DRYRUN_DOMAIN"] = dryrun_domain
    env["PHISHLET_NAME"] = phishlet_name
    # Make sure it can find the right binary
    env["EVILGINX_BIN"] = bin_path

    # Spawn with a pseudo-tty so evilginx thinks it's talking to a human.
    child = pexpect.spawn(cmd[0], env=env, timeout=10, encoding="utf-8", cwd=str(KIT_ROOT))

    try:
        # Wait for the classic evilginx ':' prompt.
        # It may print a bunch of startup noise first.
        child.expect(r":\s*", timeout=timeout)

        # Send the configuration sequence.
        commands = [
            f"config domain {dryrun_domain}",
            f"config ipv4 external 127.0.0.1",
            f"phishlets hostname {phishlet_name} {dryrun_domain}",
            f"phishlets enable {phishlet_name}",
            f"lures create {phishlet_name}",
            "lures get-url 0",
        ]

        lure_url = None
        import re
        domain_url_re = re.compile(rf"https?://[^\s]*{re.escape(dryrun_domain)}[^\s]*")

        for cmd in commands:
            child.sendline(cmd)
            time.sleep(0.35)
            if "get-url" in cmd:
                try:
                    # Prefer URLs that match the expected dry-run domain.
                    child.expect(domain_url_re, timeout=8)
                    lure_url = child.after.strip()
                except pexpect.TIMEOUT:
                    buffer = (child.before or "") + (child.after or "")
                    for line in buffer.splitlines():
                        if "http" in line.lower() and dryrun_domain in line:
                            m = domain_url_re.search(line)
                            if m:
                                lure_url = m.group(0).strip()
                                break

        if not lure_url:
            recent = (child.before or "") + (child.after or "")
            matches = domain_url_re.findall(recent)
            if matches:
                lure_url = matches[-1].strip()

        if not lure_url:
            raise EvilginxAutomationError(
                "Could not extract a lure URL from evilginx output. "
                "You may need to run the interactive dry-run once manually first, "
                "or the phishlet may have failed to load."
            )

        # We leave the child running (backgrounded from the caller's perspective).
        # The caller can keep the pexpect child object if they want to drive more later,
        # or just let it live until the script exits.
        # For the simple "quick test" flow we usually want to keep it alive.
        # Store a reference on the object so the caller can close it later.
        # Attach extra info for the caller
        child.lure_url = lure_url          # type: ignore[attr-defined]
        child.phishlet_name = phishlet_name  # type: ignore[attr-defined]
        child.dryrun_domain = dryrun_domain  # type: ignore[attr-defined]

        # Return the URL and the live pexpect child so the caller can keep
        # evilginx running for the duration of the test.
        return lure_url, child

    except pexpect.TIMEOUT as e:
        output = _safe_text(child.before) + _safe_text(child.after)
        raise EvilginxAutomationError(
            f"Timeout while configuring evilginx (after {timeout}s).\n"
            f"Last output:\n{output[-2000:]}"
        ) from e
    except pexpect.EOF as e:
        output = _safe_text(child.before) + _safe_text(child.after)
        raise EvilginxAutomationError(
            f"evilginx exited unexpectedly while we were configuring it.\n"
            f"Output:\n{output[-2000:]}"
        ) from e
