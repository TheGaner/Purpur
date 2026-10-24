#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bounded local TCP fairness experiment for the preauthentication worker quota.

This is not a production admission rate/TPS benchmark. Uses only Linux
127.0.0.1 and 127.0.0.2 loopback source addresses, no Internet traffic.
Run after cargo +1.90.0 build --release:
    python3 -B scripts/bench_admission.py
"""
from concurrent.futures import ThreadPoolExecutor
import json
import math
from pathlib import Path
import socket
import statistics
import subprocess
import tempfile
import time

from test_peer_admission import BINARY, await_status_ready, get_status


def quantiles(samples):
    assert samples
    s = sorted(samples)
    def percentile(frac):
        return round(s[math.ceil(len(s) * frac) - 1], 3)
    return {
        "count": len(s),
        "mean_ms": round(statistics.mean(s), 3),
        "p50_ms": percentile(.50),
        "p95_ms": percentile(.95),
        "p99_ms": percentile(.99),
        "max_ms": round(s[-1], 3),
    }


def rejected_status(address):
    """Assert same-source reply is refused; measure time to EOF or reset."""
    start = time.perf_counter_ns()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(1.5)
        sock.connect(address)
        # A complete protocol-777 handshake is preferable to relying only
        # on the TCP handshake, which can succeed before application reject.
        handshake = bytes([0, 0x89, 0x06, 9]) + b"localhost" + b"\x63\xdd\x01"
        sock.sendall(bytes([len(handshake)]) + handshake + b"\x01\x00")
        try:
            response = sock.recv(1)
        except ConnectionResetError:
            response = b""
        assert response == b"", f"quota unexpectedly served {response!r}"
    return (time.perf_counter_ns() - start) / 1_000_000


def trial(address):
    # Compare equivalent status request fanout without any held sockets.
    with ThreadPoolExecutor(max_workers=4) as pool:
        unloaded = list(pool.map(lambda _: get_status(address, "127.0.0.2"),
                                 range(24)))
    held = []
    try:
        for _ in range(8):
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(2)
            sock.bind(("127.0.0.1", 0))
            sock.connect(address)
            held.append(sock)
        # The accept thread has no explicit ready notification; an extra
        # saturation probe confirms that the held sockets have been admitted.
        time.sleep(.12)
        rejected = [rejected_status(address) for _ in range(8)]
        with ThreadPoolExecutor(max_workers=4) as pool:
            other = list(pool.map(lambda _: get_status(address, "127.0.0.2"),
                                  range(24)))
        assert len(other) == 24
    finally:
        for sock in held:
            sock.close()
    # Wait for EOF/permit destructors rather than asserting a fixed slot
    # release time; test a normal subsequent status request from the old IP.
    time.sleep(.12)
    with ThreadPoolExecutor(max_workers=4) as pool:
        recovered = list(pool.map(lambda _: get_status(address, "127.0.0.1"),
                                  range(12)))
    assert len(recovered) == 12
    return {
        "held_same_source": 8,
        "other_source_unloaded": quantiles(unloaded),
        "same_source_refused": quantiles(rejected),
        "other_source_success": quantiles(other),
        "original_source_recovery": quantiles(recovered),
        "unexpected_responses": 0,
    }


def main():
    assert BINARY.is_file(), f"Missing release server: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="ph-admission-bench-") as tmp:
        root = Path(tmp)
        (root / "plugins").mkdir()
        (root / "data").mkdir()
        args = [str(BINARY), "--bind", "127.0.0.1:0",
                "--max-connections", "16", "--max-per-ip", "8",
                "--plugins", str(root / "plugins"), "--data", str(root / "data")]
        server = subprocess.Popen(args, cwd=root,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, text=True, bufsize=1)
        try:
            address = await_status_ready(server)
            trials = [trial(address) for _ in range(2)]
            print(json.dumps({
                "experiment": "RV-NET-05 local preauth fairness, no gameplay",
                "global_cap": 16, "per_ip_cap": 8,
                "held_source": "127.0.0.1",
                "independent_source": "127.0.0.2",
                "concurrent_status_clients": 4,
                "per_trial": trials,
                "percentile_method": "nearest rank ceil(p*n), 1-indexed",
                "scope": "localhost, release binary, 2 trials; not TPS, load capacity or DDoS claim",
            }, indent=2))
        finally:
            try:
                out, err = server.communicate(input="stop\n", timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                out, err = server.communicate(timeout=5)
                raise AssertionError(f"server shutdown hung: {out} {err}")
            assert server.returncode == 0, (server.returncode, out, err)


if __name__ == "__main__":
    main()
