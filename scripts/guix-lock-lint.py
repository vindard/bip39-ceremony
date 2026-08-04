#!/usr/bin/env python3
"""Ensure Guix crate origins exactly mirror Cargo.lock checksums."""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "Cargo.lock"
PACKAGE = ROOT / "contrib" / "guix" / "package.scm"

CRATE_RE = re.compile(
    r'\(locked-crate\s+"(?P<name>[^"]+)"\s+"(?P<version>[^"]+)"\s+'
    r'"(?P<hash>[0-9a-z]+)"\)',
    re.MULTILINE,
)


def nix_base32(hex_digest: str) -> str:
    command = [
        "nix",
        "hash",
        "convert",
        "--hash-algo",
        "sha256",
        "--from",
        "base16",
        "--to",
        "nix32",
        hex_digest,
    ]
    return subprocess.check_output(command, text=True).strip()


def main() -> int:
    lock = tomllib.loads(LOCK.read_text())
    expected = {
        (package["name"], package["version"]): nix_base32(package["checksum"])
        for package in lock["package"]
        if "checksum" in package
    }
    declared = {
        (match["name"], match["version"]): match["hash"]
        for match in CRATE_RE.finditer(PACKAGE.read_text())
    }

    failures: list[str] = []
    for crate in sorted(expected.keys() - declared.keys()):
        failures.append(f"missing Guix crate origin: {crate[0]} {crate[1]}")
    for crate in sorted(declared.keys() - expected.keys()):
        failures.append(f"stale Guix crate origin: {crate[0]} {crate[1]}")
    for crate in sorted(expected.keys() & declared.keys()):
        if expected[crate] != declared[crate]:
            failures.append(
                f"checksum mismatch for {crate[0]} {crate[1]}: "
                f"expected {expected[crate]}, found {declared[crate]}"
            )

    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"Guix crate origins match {len(expected)} Cargo.lock packages")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
