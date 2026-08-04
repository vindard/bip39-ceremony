#!/usr/bin/env python3
"""Launch a release binary in a pseudo-terminal and confirm clean exit."""

from __future__ import annotations

import fcntl
import os
import pty
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path


def smoke(binary: Path) -> None:
    master, slave = pty.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 120, 0, 0))
        process = subprocess.Popen(
            [binary],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            close_fds=True,
            env=os.environ | {"TERM": "xterm-256color", "NO_COLOR": "1"},
        )
    finally:
        os.close(slave)

    try:
        time.sleep(0.2)
        os.write(master, b"q")
        time.sleep(0.1)
        os.write(master, b"q")
        try:
            status = process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise RuntimeError("did not exit through the confirmation flow") from None
        if status != 0:
            raise RuntimeError(f"exited with status {status}")
    finally:
        os.close(master)


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} BINARY", file=sys.stderr)
        return 2
    binary = Path(sys.argv[1]).resolve()
    if not binary.is_file():
        print(f"not a file: {binary}", file=sys.stderr)
        return 2
    try:
        smoke(binary)
    except RuntimeError as error:
        print(f"PTY smoke failed for {binary}: {error}", file=sys.stderr)
        return 1
    print(f"PTY smoke passed: {binary}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
