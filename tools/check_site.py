#!/usr/bin/env python3
"""Refuse a built documentation site that is missing a required page.

The site directory is the output of ``mkdocs build --strict``. This
script reads that output. It does not build the site.
"""

import sys
from pathlib import Path


def main(argv):
    if len(argv) != 1:
        print("usage: check_site.py SITE", file=sys.stderr)
        return 2
    site = Path(argv[0])
    index = site / "index.html"
    failures = []
    if not index.is_file():
        failures.append(f"missing {index}")
    else:
        text = index.read_text(encoding="utf-8", errors="replace")
        if 'id="tutorial-palette-toggle"' not in text:
            failures.append("front page has no palette switch")
        if "tutorial-dark-palette" not in text:
            failures.append("front page does not load the dark palette")
    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
