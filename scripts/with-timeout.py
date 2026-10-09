#!/usr/bin/env python3
"""Run one command with a wall-clock limit and kill its entire process group."""

import contextlib
import os
import signal
import subprocess
import sys


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: with-timeout.py SECONDS COMMAND [ARG ...]", file=sys.stderr)
        return 2
    try:
        seconds = int(sys.argv[1])
    except ValueError:
        print("timeout must be a positive integer", file=sys.stderr)
        return 2
    if seconds <= 0:
        print("timeout must be a positive integer", file=sys.stderr)
        return 2
    child = subprocess.Popen(sys.argv[2:], start_new_session=True)
    try:
        return child.wait(timeout=seconds)
    except subprocess.TimeoutExpired:
        with contextlib.suppress(ProcessLookupError):
            os.killpg(child.pid, signal.SIGKILL)
        child.wait()
        print(f"command exceeded {seconds} seconds", file=sys.stderr)
        return 124


if __name__ == "__main__":
    raise SystemExit(main())
