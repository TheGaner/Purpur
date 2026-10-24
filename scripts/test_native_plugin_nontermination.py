#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Demonstrate (NOT fix) native plugin callback deadlock in isolated child.

The .so fixture is compiled in a temporary directory and is NEVER loaded
into the user's real server. Only a child that this test starts is killed.
"""
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "target" / "release" / "purpur-horizon"
C_SOURCE = r"""
#include <stdio.h>
#include <unistd.h>
unsigned int horizon_plugin_abi_version(void) { return 1; }
int horizon_plugin_event(unsigned int event, unsigned long long tick,
                         const char *payload) {
    (void)tick; (void)payload;
    if (event == 1) {
        puts("RVPLUG_STUCK_ENABLE_REACHED");
        fflush(stdout);
        for (;;) pause();
    }
    return 0;
}
"""


def main():
    assert SERVER.is_file(), "Build the release server before this fixture."
    with tempfile.TemporaryDirectory(prefix="ph-native-stall-") as temp:
        root = Path(temp)
        plugins = root / "plugins"
        native = plugins / "rust"
        native.mkdir(parents=True)
        data = root / "data"
        args = [str(SERVER), "--bind", "127.0.0.1:0",
                "--plugins", str(plugins), "--data", str(data)]

        # Healthy no-plugin launch/shutdown must work, demonstrating that
        # inability to stop below is specifically due to the native callback.
        healthy = subprocess.run(
            args, cwd=root, input="stop\n", text=True,
            capture_output=True, timeout=5)
        assert healthy.returncode == 0, (
            healthy.returncode, healthy.stdout, healthy.stderr)
        assert "Purpur-Horizon standalone server at" in healthy.stdout, (
            healthy.stdout, healthy.stderr)

        library = native / "libnever_return.so"
        build = subprocess.run(
            ["cc", "-std=c11", "-O0", "-shared", "-fPIC", "-x", "c",
             "-", "-o", str(library)],
            input=C_SOURCE, capture_output=True, text=True, cwd=root,
            timeout=8)
        assert build.returncode == 0, build.stderr

        child = subprocess.Popen(
            args, cwd=root, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        started = time.monotonic()
        stalled = False
        try:
            try:
                out, err = child.communicate("stop\n", timeout=1.2)
                raise AssertionError(
                    "Expected in-process plugin callback to block stop: "
                    f"exit={child.returncode} stdout={out} stderr={err}")
            except subprocess.TimeoutExpired:
                stalled = True
                child.kill()
                out, err = child.communicate(timeout=4)
            assert "RVPLUG_STUCK_ENABLE_REACHED" in out, (out, err)
            assert "Purpur-Horizon standalone server at" not in out, (
                "native plugin enable blocked before accept-loop readiness", out, err)
            assert child.returncode != 0, child.returncode
        finally:
            if child.poll() is None:
                child.kill()
                child.communicate(timeout=4)
        assert stalled
        print("CONFIRMED EXPOSURE (not protection): native enable callback "
              "prevented clean server shutdown; disposable test child killed/reaped; "
              f"elapsed={time.monotonic() - started:.3f}s")
        print("PASS: no-plugin control completed normally; test has no residual process")


if __name__ == "__main__":
    main()
