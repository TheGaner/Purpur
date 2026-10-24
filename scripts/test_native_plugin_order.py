#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Assert deterministic native loading and first-winner command ownership.

Requires a compiled release server and C compiler. Temporary trusted test
libraries are loaded only by short-lived disposable server processes.
"""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "target/release/purpur-horizon"
C_SOURCE = r"""
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdio.h>
uint32_t horizon_plugin_abi_version(void) { return 1; }
int horizon_plugin_event(uint32_t kind, uint64_t tick, const char *payload) {
    (void)tick;
    (void)payload;
    if (kind == 3) {
        puts("ORDER_WIN_@MARKER@");
        fflush(stdout);
    }
    return 0;
}
size_t horizon_plugin_commands_v2(unsigned char *out, size_t capacity) {
    const char manifest[] = "collision|example.collision|";
    size_t size = sizeof(manifest) - 1;
    if (capacity >= size) memcpy(out, manifest, size);
    return size;
}
"""


def trial(creation_order):
    with tempfile.TemporaryDirectory(prefix="ph-plugin-order-") as dirname:
        root = Path(dirname)
        rust = root / "plugins" / "rust"
        rust.mkdir(parents=True)
        for letter in creation_order:
            lib = rust / f"lib{letter}.so"
            build = subprocess.run(
                ["cc", "-O0", "-shared", "-fPIC", "-x", "c", "-", "-o", str(lib)],
                input=C_SOURCE.replace("@MARKER@", letter.upper()),
                capture_output=True, text=True, timeout=8)
            assert build.returncode == 0, build.stderr
        server = subprocess.run(
            [str(SERVER), "--bind", "127.0.0.1:0",
             "--plugins", str(root / "plugins"), "--data", str(root / "data")],
            input="collision\nstop\n", capture_output=True, text=True,
            cwd=root, timeout=8)
        assert server.returncode == 0, (server.stdout, server.stderr)
        lines = [line for line in server.stdout.splitlines()
                 if line.startswith("[rust] loaded ")]
        assert lines == ["[rust] loaded liba.so", "[rust] loaded libz.so"], (
            creation_order, lines, server.stderr)
        assert server.stdout.count("ORDER_WIN_A") == 1, (server.stdout, server.stderr)
        assert "ORDER_WIN_Z" not in server.stdout, server.stdout
        assert "invalid command" not in server.stderr
        assert "duplicate" in server.stderr.lower(), server.stderr
        print(f"PASS: creation={creation_order}, loading=a,z, first-owner=A")


def main():
    assert SERVER.is_file(), "Build release server first"
    trial(("z", "a"))
    trial(("a", "z"))
    trial(("z", "a"))


if __name__ == "__main__":
    main()
