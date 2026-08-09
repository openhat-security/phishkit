#!/usr/bin/env python3
"""Removed: use the native CLI wizards instead.

  make cli
  ./target/release/phishkit wiz quickstart
  ./target/release/phishkit wiz send
"""
from __future__ import annotations

import sys


def main() -> int:
    print(
        "scripts/setup_campaign.py has been removed.\n"
        "Use:  make cli && ./target/release/phishkit wiz quickstart\n"
        "Or:   make desktop",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
