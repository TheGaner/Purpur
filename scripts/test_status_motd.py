#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Real protocol-777 status JSON string limits and startup preflight.

Run after cargo +1.90.0 build --release. Only uses localhost and
temporary directories. Tests invalid configuration without opening TCP.
"""
import json
from pathlib import Path
import re
import socket
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "target/release/purpur-horizon"


def varint(number):
    result = bytearray()
    while number >= 128:
        result.append((number & 127) | 128)
        number >>= 7
    result.append(number)
    return bytes(result)


def read_exact(sock, n):
    data = bytearray()
    while len(data) < n:
        part = sock.recv(n - len(data))
        if not part:
            raise AssertionError("truncated TCP status reply")
        data.extend(part)
    return bytes(data)


def read_varint(sock):
    n = 0
    for shift in range(0, 35, 7):
        digit = read_exact(sock, 1)[0]
        n |= (digit & 127) << shift
        if digit < 128:
            return n
    raise AssertionError("oversized VarInt")


def status_reply(port):
    with socket.create_connection(("127.0.0.1", port), timeout=2) as sock:
        sock.settimeout(2)
        hostname = b"localhost"
        hs = b"\x00" + varint(777) + varint(len(hostname)) + hostname + b"\x63\xdd\x01"
        sock.sendall(varint(len(hs)) + hs + b"\x01\x00")
        length = read_varint(sock)
        assert length <= 32767 + 16, length
        body = read_exact(sock, length)
    assert body[0] == 0, body[:4]
    index = 1
    json_len = 0
    for shift in range(0, 35, 7):
        digit = body[index]
        index += 1
        json_len |= (digit & 127) << shift
        if digit < 128:
            break
    encoded = body[index:]
    assert len(encoded) == json_len
    assert len(encoded) <= 32767, len(encoded)
    return json.loads(encoded)


def main():
    assert SERVER.is_file(), f"Release server missing: {SERVER}"
    with tempfile.TemporaryDirectory(prefix="ph-motd-") as temp:
        root = Path(temp)
        args = [str(SERVER), "--bind", "127.0.0.1:0",
                "--plugins", str(root / "plugins"), "--data", str(root / "data")]
        too_large = "x" * 4097
        invalid = subprocess.run(
            args + ["--motd", too_large],
            input="stop\n", text=True, capture_output=True, cwd=root,
            timeout=5)
        assert invalid.returncode != 0, invalid.stdout
        assert "--motd exceeds 4096 UTF-8 bytes" in invalid.stderr, invalid.stderr
        assert "standalone server at" not in invalid.stdout, invalid.stdout
        print("PASS: 4097-byte MOTD rejected before server bind")

        for motd in ("x" * 4096, "\x01" * 4096, "é" * 2048):
            child = subprocess.Popen(
                args + ["--motd", motd],
                cwd=root, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True, bufsize=1)
            try:
                opening = child.stdout.readline()
                port = re.search(r"127\.0\.0\.1:(\d+)", opening)
                assert port, (opening, child.poll())
                status = status_reply(int(port.group(1)))
                assert status["description"]["text"] == motd
                assert status["version"]["protocol"] == 777
            finally:
                try:
                    out, err = child.communicate("stop\n", timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.communicate(timeout=5)
                    raise AssertionError("valid-MOTD fixture server hung on shutdown")
                assert child.returncode == 0, (out, err)
        print("PASS: 4096-byte ASCII/control/UTF-8 MOTDs served as valid bounded protocol 777 JSON")


if __name__ == "__main__":
    main()
