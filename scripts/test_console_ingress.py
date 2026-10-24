#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bounded console input regression against an isolated release process.

An oversized line starting with a privileged command must not be trimmed
and executed, nor consume/drop valid lines after it.
"""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "target/release/purpur-horizon"


def main():
    assert SERVER.is_file(), f"Missing release server: {SERVER}"
    with tempfile.TemporaryDirectory(prefix="ph-console-ingress-") as temp:
        root = Path(temp)
        args = [str(SERVER), "--bind", "127.0.0.1:0",
                "--plugins", str(root / "plugins"),
                "--data", str(root / "data")]
        oversized = b"stop" + b" " * 300_000 + b"\n"
        request = oversized + b"version\nstop\n"
        proc = subprocess.run(
            args, input=request, cwd=root, capture_output=True, timeout=7)
        out = proc.stdout.decode("utf-8", errors="replace")
        err = proc.stderr.decode("utf-8", errors="replace")
        assert proc.returncode == 0, (proc.returncode, out, err)
        assert "[console] command exceeds 4096 UTF-8 bytes; ignored" in err, err
        assert out.count("Purpur-Horizon 0.1.0 (command prototype)") == 1, out
        # A zero-length command is harmless; no extra command from the
        # 300KiB discarded tail should reach the dispatcher.
        assert "[command] Unknown command:" not in err, err
        print("PASS: oversized 300KiB 'stop'+spaces discarded without executing; "
              "following version and stop commands executed in order")


if __name__ == "__main__":
    main()
