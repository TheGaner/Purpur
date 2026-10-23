#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Real TCP preauth fairness regression: one source cannot exhaust all worker slots."""
import json
import re
import socket
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "release" / "purpur-horizon"


def varint(n: int) -> bytes:
    out = bytearray()
    while n >= 128:
        out.append((n & 127) | 128)
        n >>= 7
    out.append(n)
    return bytes(out)


def recv_exact(sock, size):
    out = bytearray()
    while len(out) < size:
        chunk = sock.recv(size - len(out))
        if not chunk:
            raise AssertionError("peer unexpectedly closed response")
        out.extend(chunk)
    return bytes(out)


def get_status(address, source=None):
    started = time.perf_counter_ns()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as client:
        client.settimeout(2)
        if source:
            client.bind((source, 0))
        client.connect(address)
        host = b"localhost"
        handshake = b"\x00" + varint(777) + varint(len(host)) + host + b"\x63\xdd\x01"
        client.sendall(varint(len(handshake)) + handshake + b"\x01\x00")
        frame_len = 0
        for index in range(5):
            octet = recv_exact(client, 1)[0]
            frame_len |= (octet & 127) << (7 * index)
            if not octet & 128:
                break
        assert frame_len < 1024, frame_len
        body = recv_exact(client, frame_len)
        assert body[:1] == b"\x00", body[:5]
        # Encoded JSON follows packet id and string-length VarInt.
        pos = 1
        size = 0
        for index in range(5):
            octet = body[pos]
            pos += 1
            size |= (octet & 127) << (7 * index)
            if not octet & 128:
                break
        answer = json.loads(body[pos:pos + size])
        assert answer["version"]["protocol"] == 777
    return (time.perf_counter_ns() - started) / 1_000_000


def main():
    assert BINARY.is_file(), "Build the release server first"
    with tempfile.TemporaryDirectory(prefix="ph-peer-") as tmp:
        root = Path(tmp)
        (root / "plugins").mkdir()
        (root / "data").mkdir()
        args = [str(BINARY), "--bind", "127.0.0.1:0",
                "--max-connections", "3", "--max-per-ip", "1",
                "--plugins", str(root / "plugins"), "--data", str(root / "data")]
        for bad in ("0", "1025"):
            probe = subprocess.run(
                [*args[:5], "--max-per-ip", bad, *args[7:]],
                text=True, capture_output=True, input="", timeout=5, cwd=root)
            assert probe.returncode != 0, (bad, probe.stdout, probe.stderr)

        server = subprocess.Popen(args, cwd=root, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  text=True, bufsize=1)
        held = None
        try:
            line = server.stdout.readline()
            match = re.search(r"127\.0\.0\.1:(\d+)", line)
            assert match, (line, server.poll())
            address = ("127.0.0.1", int(match.group(1)))
            held = socket.create_connection(address, timeout=2)
            time.sleep(.15)  # ensure first connection occupies the single peer slot
            refused = socket.create_connection(address, timeout=2)
            refused.settimeout(.8)
            try:
                response = refused.recv(1)
                assert response == b"", response
            except ConnectionResetError:
                pass
            finally:
                refused.close()

            # Linux loopback routes all 127/8 addresses locally. Different
            # actual peer address must still be allowed a handshake slot.
            get_status(address, source="127.0.0.2")
            held.close()
            held = None
            time.sleep(.15)
            get_status(address)
            print("PASS: same-IP overflow refused; different-IP status works; slot reclaimed; invalid limits rejected")
        finally:
            if held:
                held.close()
            try:
                stdout, stderr = server.communicate(input="stop\n", timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                stdout, stderr = server.communicate(timeout=5)
                raise AssertionError(f"server failed to stop: {stdout}\n{stderr}")
            assert server.returncode == 0, (server.returncode, stdout, stderr)


if __name__ == "__main__":
    main()
