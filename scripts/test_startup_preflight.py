#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Store ownership and data integrity must preflight before plugin enable/listen.

Run after release build. Every server and C test plugin is disposable and
scoped to a temporary directory; no real plugin or store is modified.
"""
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "target/release/purpur-horizon"
SOURCE = r"""
#include <stdint.h>
#include <stdio.h>
uint32_t horizon_plugin_abi_version(void) { return 1; }
int horizon_plugin_event(uint32_t code, uint64_t tick, const char *payload) {
    (void)tick; (void)payload;
    if (code == 1) {
        puts("RV_BOOT_PLUGIN_ENABLED");
        fflush(stdout);
    }
    return 0;
}
"""


def launch_args(root, data):
    return [str(SERVER), "--bind", "127.0.0.1:0",
            "--plugins", str(root / "plugins"), "--data", str(data)]


def assert_no_boot_effects(stdout):
    assert "standalone server at" not in stdout, stdout
    assert "RV_BOOT_PLUGIN_ENABLED" not in stdout, stdout


def main():
    assert SERVER.is_file(), "Build the release server first"
    with tempfile.TemporaryDirectory(prefix="ph-startup-preflight-") as folder:
        root = Path(folder)
        rust = root / "plugins" / "rust"
        rust.mkdir(parents=True)
        compiled = subprocess.run(
            ["cc", "-std=c11", "-shared", "-fPIC", "-x", "c", "-", "-o",
             str(rust / "libboot.so")],
            input=SOURCE, text=True, capture_output=True, timeout=8,
        )
        assert compiled.returncode == 0, compiled.stderr
        data = root / "data"
        args = launch_args(root, data)

        # A first server proves the fixture plugin actually runs on startup.
        with (root / "owner.log").open("w+") as log:
            owner = subprocess.Popen(
                args, stdin=subprocess.PIPE, stdout=log, stderr=log,
                text=True, cwd=root)
            try:
                for _ in range(80):
                    log.flush()
                    content = (root / "owner.log").read_text()
                    if "Native Rust permissions and utilities: enabled" in content:
                        break
                    assert owner.poll() is None, content
                    time.sleep(0.025)
                else:
                    raise AssertionError("first server did not finish startup")
                assert "RV_BOOT_PLUGIN_ENABLED" in content, content
                # A bound TCP port is NOT a ready server. A finite native
                # enable callback must finish before the address banner.

                loser = subprocess.run(
                    args, input="stop\n", text=True, capture_output=True,
                    cwd=root, timeout=6,
                )
                assert loser.returncode != 0, loser.stdout
                assert "store already open by another writer" in loser.stderr, loser.stderr
                assert_no_boot_effects(loser.stdout)
                assert owner.poll() is None, "losing process affected the owner"
                print("PASS: concurrent second writer refused before plugin enable/listener")
            finally:
                if owner.poll() is None:
                    try:
                        owner.communicate("stop\n", timeout=5)
                    except subprocess.TimeoutExpired:
                        owner.kill()
                        owner.communicate(timeout=4)
                assert owner.returncode == 0, (root / "owner.log").read_text()
                completed = (root / "owner.log").read_text()
                assert "Purpur-Horizon standalone server at" in completed
                assert completed.index("RV_BOOT_PLUGIN_ENABLED") < completed.index(
                    "Purpur-Horizon standalone server at"), completed

        corrupt = root / "invalid-data"
        corrupt.mkdir()
        (corrupt / "utilities.json").write_text("{not valid JSON")
        bad = subprocess.run(
            launch_args(root, corrupt), input="stop\n", capture_output=True,
            text=True, cwd=root, timeout=6,
        )
        assert bad.returncode != 0, (bad.stdout, bad.stderr)
        assert "expected" in bad.stderr.lower() or "key" in bad.stderr.lower(), bad.stderr
        assert_no_boot_effects(bad.stdout)
        print("PASS: corrupted persistent utilities data rejected before plugin enable/listener")


if __name__ == "__main__":
    main()
