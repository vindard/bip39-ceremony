#!/usr/bin/env python3
"""Reject likely private local details and credentials from tracked content."""

from pathlib import Path
import re
import subprocess
import sys

SKIPPED_PREFIXES = (".git/", "target/")
PATTERNS = {
    "absolute home path": re.compile(r"/(?:home|Users)/[^/\s]+/"),
    "private key": re.compile(r"BEGIN [A-Z ]*PRIVATE KEY"),
    "GitHub token": re.compile(r"\bgh[opusr]_[A-Za-z0-9]{20,}\b"),
    "assigned credential": re.compile(
        r"(?i)\b(?:api[_-]?key|access[_-]?token|secret[_-]?key|password)\s*[:=]\s*\S+"
    ),
}


def tracked_files() -> list[Path]:
    output = subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard"], text=True)
    return [Path(line) for line in output.splitlines() if line]


def main() -> int:
    findings: list[str] = []
    for path in tracked_files():
        name = path.as_posix()
        if name.startswith(SKIPPED_PREFIXES) or not path.is_file():
            continue
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            continue
        for label, pattern in PATTERNS.items():
            for match in pattern.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                findings.append(f"{name}:{line}: {label}")

    if findings:
        print("privacy lint failed:", file=sys.stderr)
        print("\n".join(findings), file=sys.stderr)
        return 1
    print("privacy lint passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
