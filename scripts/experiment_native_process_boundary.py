#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review-only experiment: native callbacks in disposable separate processes.

This is NOT production plugin IPC, syscall sandboxing or host integration.
It compiles trusted test-only C and supervises only children it creates.
Usage: python3 -B scripts/experiment_native_process_boundary.py
"""
import ctypes
import os
from pathlib import Path
import resource
import selectors
import signal
import subprocess
import sys
import tempfile
import time

SOURCE = r"""
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <string.h>
#include <unistd.h>
uint32_t horizon_plugin_abi_version(void) { return 1; }
int horizon_plugin_event(uint32_t event, uint64_t tick, const char *payload) {
    (void)tick;
    if (event != 1) return -1;
    if (strcmp(payload, "hang") == 0) {
        for (;;) pause();
    }
    if (strcmp(payload, "crash") == 0) {
        *(volatile int *)0 = 1;
        return -2;
    }
    if (strcmp(payload, "normal") == 0) return 7;
    if (strcmp(payload, "filesystem") == 0) {
        const char *target = getenv("PH_TEST_ONLY_TARGET");
        if (!target) return -4;
        FILE *f = fopen(target, "w");
        if (!f) return -5;
        fputs("CHILD_COULD_WRITE_HOST_FILE", f);
        fclose(f);
        return 7;
    }
    return -3;
}
"""


def worker(library, mode):
    # Keep deliberate SIGSEGV in an independent child, with no core dump
    # saved to disk. No external or real-world plugin is ever loaded.
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    signal.signal(signal.SIGSEGV, signal.SIG_DFL)
    dll = ctypes.CDLL(library)
    dll.horizon_plugin_abi_version.restype = ctypes.c_uint32
    if dll.horizon_plugin_abi_version() != 1:
        raise RuntimeError("unexpected ABI")
    event = dll.horizon_plugin_event
    event.restype = ctypes.c_int
    event.argtypes = [ctypes.c_uint32, ctypes.c_uint64, ctypes.c_char_p]
    print(f"WORKER_ENTER_{mode}", flush=True)
    value = event(1, 0, mode.encode("ascii"))
    print(f"WORKER_RETURN_{value}", flush=True)
    if value != 7:
        raise AssertionError(f"unexpected result {value}")


def invoke(library, mode, timeout, env=None):
    child = subprocess.Popen(
        [sys.executable, "-B", str(Path(__file__).resolve()),
         "--worker", str(library), mode],
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, start_new_session=True, env=env,
    )
    started = None
    timed_out = False
    stdout = b""
    stderr = b""
    try:
        # Time the callback only AFTER the independent worker has loaded its
        # library and written the flushed entry marker. Python startup time
        # is not part of the native-event responsiveness budget.
        with selectors.DefaultSelector() as ready:
            ready.register(child.stdout, selectors.EVENT_READ)
            assert ready.select(timeout=3), "worker never reached callback"
        first = os.read(child.stdout.fileno(), 4096)
        assert f"WORKER_ENTER_{mode}\n".encode() in first, first
        stdout += first
        started = time.monotonic()
        try:
            rest, stderr = child.communicate(timeout=timeout)
            stdout += rest
        except subprocess.TimeoutExpired:
            timed_out = True
            # Own worker's session only. Not a generic guarantee against
            # an adversarial child intentionally escaping its process group.
            os.killpg(child.pid, signal.SIGKILL)
            rest, stderr = child.communicate(timeout=3)
            stdout += rest
    finally:
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.communicate(timeout=3)
    assert started is not None
    elapsed = time.monotonic() - started
    assert child.poll() is not None, "child was not reaped"
    assert f"WORKER_ENTER_{mode}".encode() in stdout, (stdout, stderr)
    if mode in ("normal", "filesystem"):
        assert not timed_out and child.returncode == 0, (stdout, stderr)
        assert b"WORKER_RETURN_7" in stdout
    elif mode == "crash":
        assert not timed_out and child.returncode == -signal.SIGSEGV, (
            child.returncode, stdout, stderr)
    else:
        assert mode == "hang" and timed_out
        assert child.returncode == -signal.SIGKILL, (
            child.returncode, stdout, stderr)
        assert b"WORKER_RETURN" not in stdout
    return elapsed


def main():
    with tempfile.TemporaryDirectory(prefix="ph-native-process-probe-") as temp:
        library = Path(temp) / "libtest_only.so"
        compiled = subprocess.run(
            ["cc", "-std=c11", "-fPIC", "-shared", "-O0",
             "-x", "c", "-", "-o", str(library)],
            input=SOURCE, text=True, capture_output=True, timeout=10,
        )
        assert compiled.returncode == 0, compiled.stderr
        for run in (1, 2):
            target = Path(temp) / f"host-only-sentinel-{run}.txt"
            for mode in ("normal", "crash", "hang", "filesystem"):
                env = os.environ.copy()
                if mode == "filesystem":
                    env["PH_TEST_ONLY_TARGET"] = str(target)
                elapsed = invoke(library, mode, 0.8, env=env)
                if mode == "filesystem":
                    assert target.read_text() == "CHILD_COULD_WRITE_HOST_FILE", target
                print(f"PASS trial={run} mode={mode} "
                      f"elapsed={elapsed:.3f}s supervisor=alive worker=reaped",
                      flush=True)
    print("RESULT: process boundary contains test crash/hang cases, "
          "BUT child retains host-user filesystem access; "
          "NOT a secure native plugin sandbox or server integration.")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        worker(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 1:
        main()
    else:
        raise SystemExit("Invalid arguments")
