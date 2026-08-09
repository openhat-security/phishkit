#!/usr/bin/env python3
"""Removed: use the phishkit CLI instead.

  make cli
  ./target/release/phishkit list-captures -p <profile>
  ./target/release/phishkit delete-capture -p <profile> -s <session-id>

  Or: make session-list PROFILE=… / make session-delete PROFILE=… ID=…
"""
from __future__ import annotations

import sys


def main() -> int:
    print(
        "scripts/delete_session.py has been removed.\n"
        "Use:  make session-list PROFILE=<id>\n"
        "      make session-delete PROFILE=<id> ID=<session#>\n"
        "Or:   ./target/release/phishkit list-captures -p <id>\n"
        "      ./target/release/phishkit delete-capture -p <id> -s <session#>",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
