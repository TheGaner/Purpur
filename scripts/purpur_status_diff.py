#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Differential Minecraft 26.3 protocol-777 status/ping probe.

Actual Purpur parity requires --reference HOST:PORT pointing to an
operator-authorized *running* Purpur server with recorded build/config.
This does not download/run a server, accept an EULA, or probe arbitrary
Internet hosts automatically.

Without --reference, --self-check tests ONLY the comparison transport by
probing the current Rust release server twice. A self-check is NOT evidence
of Purpur behavioral equivalence.

Examples:
  python3 -B scripts/purpur_status_diff.py --self-check
  python3 -B scripts/purpur_status_diff.py --reference 127.0.0.1:25565 \
      --output /tmp/ph-status-differential.json
  python3 -B scripts/purpur_status_diff.py --reference 127.0.0.1:25565 \
      --allow-difference version.name --allow-difference players.max \
      --allow-difference description --output /tmp/ph-parity.json
"""
import argparse
from contextlib import contextmanager
import json
from pathlib import Path
import socket
import struct
import subprocess
import sys
import tempfile
from typing import Any

from test_peer_admission import await_status_ready

ROOT = Path(__file__).resolve().parents[1]
RELEASE_BINARY = ROOT / "target/release/purpur-horizon"
SOURCE_COMMIT = "5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0"
REFERENCE_BUILD = "26.3.build.2654"
PROTOCOL = 777
MAX_PACKET_BYTES = 1024 * 1024
KEY_PATHS = ("version.protocol", "version.name", "players.max",
             "players.online", "description")


def endpoint(value: str) -> tuple[str, int]:
    """Explicit endpoint only; no discovery, broadcasting or network scan."""
    if not value or ":" not in value:
        raise ValueError("expected HOST:PORT")
    host, port_text = value.rsplit(":", 1)
    if not host or not port_text.isdecimal():
        raise ValueError("expected HOST:PORT")
    port = int(port_text)
    if not 1 <= port <= 65535:
        raise ValueError("port must be 1..65535")
    return host, port


def varint(value: int) -> bytes:
    if value < 0:
        raise ValueError("negative packet length")
    result = bytearray()
    while value >= 128:
        result.append((value & 0x7F) | 0x80)
        value >>= 7
    result.append(value)
    return bytes(result)


def read_exact(stream: socket.socket, size: int) -> bytes:
    received = bytearray()
    while len(received) < size:
        chunk = stream.recv(size - len(received))
        if not chunk:
            raise EOFError(f"short frame: wanted {size}, got {len(received)}")
        received.extend(chunk)
    return bytes(received)


def read_varint(stream: socket.socket) -> int:
    number = 0
    for index in range(5):
        b = read_exact(stream, 1)[0]
        if index == 4 and b > 0x0F:
            raise ValueError("VarInt overflow")
        number |= (b & 0x7F) << (7 * index)
        if b < 0x80:
            return number
    raise ValueError("overlong VarInt")


def frame(payload: bytes) -> bytes:
    return varint(len(payload)) + payload


def probe_status(host: str, port: int, *, timeout: float = 4.0) -> dict:
    """Fetch status and pong over one fresh TCP connection."""
    with socket.create_connection((host, port), timeout=timeout) as stream:
        stream.settimeout(timeout)
        host_bytes = host.encode("utf-8")
        if len(host_bytes) > 255:
            raise ValueError("status hostname exceeds 255 UTF-8 bytes")
        handshake = (b"\x00" + varint(PROTOCOL) + varint(len(host_bytes))
                     + host_bytes + struct.pack(">H", port) + b"\x01")
        stream.sendall(frame(handshake) + frame(b"\x00"))
        size = read_varint(stream)
        if not 1 <= size <= MAX_PACKET_BYTES:
            raise ValueError("status frame exceeds parser limit")
        # Packet ID 0 is one byte; bounded JSON read for independent server.
        payload = read_exact(stream, size)
        if not payload or payload[0] != 0:
            raise ValueError("not a status response packet")
        index = 1
        json_size = 0
        for i in range(5):
            if index >= len(payload):
                raise ValueError("truncated JSON length VarInt")
            b = payload[index]
            index += 1
            if i == 4 and b > 0x0F:
                raise ValueError("JSON length VarInt overflow")
            json_size |= (b & 0x7F) << (7 * i)
            if not b & 0x80:
                break
        else:
            raise ValueError("JSON length VarInt too long")
        if json_size != len(payload) - index:
            raise ValueError("declared JSON length differs from received bytes")
        status = json.loads(payload[index:].decode("utf-8"))
        validate_status(status)

        # The ping is a separate packet; do not confuse it with status.
        token = 0x1234567890ABCDEF
        ping = b"\x01" + struct.pack(">q", token)
        stream.sendall(frame(ping))
        pong_size = read_varint(stream)
        if pong_size != 9:
            raise ValueError(f"expected 9-byte pong, got {pong_size}")
        pong = read_exact(stream, pong_size)
        if pong != ping:
            raise ValueError("ping payload did not round-trip unchanged")
    return status


def validate_status(status: Any) -> None:
    if not isinstance(status, dict):
        raise ValueError("status response must be a JSON object")
    version = status.get("version")
    players = status.get("players")
    if not isinstance(version, dict) or not isinstance(players, dict):
        raise ValueError("missing version/players objects")
    if not isinstance(version.get("name"), str):
        raise ValueError("version.name must be a string")
    if type(version.get("protocol")) is not int:
        raise ValueError("version.protocol must be an integer")
    for key in ("max", "online"):
        if type(players.get(key)) is not int or players[key] < 0:
            raise ValueError(f"players.{key} must be a nonnegative integer")
    if "description" not in status:
        raise ValueError("missing description")
    if not isinstance(status["description"], (dict, list, str)):
        raise ValueError("description is not a text component")


def pick(status: dict, key: str) -> Any:
    value = status
    for part in key.split("."):
        value = value[part]
    return value


def compare_status(reference: dict, candidate: dict,
                   allowed: frozenset[str] = frozenset()) -> dict:
    validate_status(reference)
    validate_status(candidate)
    wrong = sorted(allowed - set(KEY_PATHS))
    if wrong:
        raise ValueError(f"unknown allow-difference field(s): {wrong}")
    diffs = []
    for path in KEY_PATHS:
        left, right = pick(reference, path), pick(candidate, path)
        if left != right:
            diffs.append({
                "field": path, "reference": left, "candidate": right,
                "accepted_deviation": path in allowed,
            })
    return {
        "common_fields": list(KEY_PATHS),
        "matches": len(KEY_PATHS) - len(diffs),
        "differences": diffs,
        "unapproved_differences": sum(
            not item["accepted_deviation"] for item in diffs),
        "reference_optional_fields": sorted(set(reference) - {
            "version", "players", "description"}),
        "candidate_optional_fields": sorted(set(candidate) - {
            "version", "players", "description"}),
    }


@contextmanager
def started_candidate():
    if not RELEASE_BINARY.is_file():
        raise FileNotFoundError(
            f"release binary missing: {RELEASE_BINARY}; build with cargo +1.90.0 build --release")
    with tempfile.TemporaryDirectory(prefix="ph-purpur-status-") as scratch:
        root = Path(scratch)
        child = subprocess.Popen(
            [str(RELEASE_BINARY), "--bind", "127.0.0.1:0",
             "--data", str(root / "data"), "--plugins", str(root / "plugins")],
            cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, bufsize=0,
        )
        try:
            addr = await_status_ready(child, timeout=5)
            yield addr
        finally:
            try:
                child.communicate(b"stop\n", timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.communicate(timeout=5)
                raise AssertionError("Rust status candidate failed to stop")
            if child.returncode != 0:
                raise AssertionError(
                    f"Rust status candidate exited {child.returncode}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--reference", metavar="HOST:PORT",
                       help="existing operator-authorized Purpur 26.3 server")
    group.add_argument("--self-check", action="store_true",
                       help="only verifies harness plumbing using Rust on both sides")
    parser.add_argument("--candidate", metavar="HOST:PORT",
                        help="existing running Rust server; otherwise starts release build")
    parser.add_argument("--allow-difference", action="append", default=[],
                        choices=KEY_PATHS, help="explicitly classified divergence")
    parser.add_argument("--output", type=Path,
                        help="write complete machine-readable status and differences")
    args = parser.parse_args()
    if args.self_check and args.candidate:
        parser.error("--self-check starts its own disposable Rust server")
    if args.self_check:
        with started_candidate() as addr:
            reference = probe_status(*addr)
            candidate = probe_status(*addr)
        ref_endpoint = "same disposable Rust server (NOT Purpur)"
    else:
        ref_host, ref_port = endpoint(args.reference)
        reference = probe_status(ref_host, ref_port)
        if args.candidate:
            candidate = probe_status(*endpoint(args.candidate))
        else:
            with started_candidate() as addr:
                candidate = probe_status(*addr)
        ref_endpoint = args.reference

    comparison = compare_status(reference, candidate,
                                frozenset(args.allow_difference))
    record = {
        "mode": "harness-only-self-check" if args.self_check else "external-reference-probe",
        "reference_endpoint": ref_endpoint,
        "claimed_reference": {"project": "Purpur", "minecraft_version": "26.3",
                              "build": "2654", "source_commit": SOURCE_COMMIT},
        "reference_identity_verified_by_tool": False,
        "operator_must_confirm_reference_build_config": not args.self_check,
        "observed_common_fields_match": comparison["unapproved_differences"] == 0,
        "purpur_parity_verified": False,
        "reference_status": reference,
        "candidate_status": candidate,
        "comparison": comparison,
        "note": ("The self-check is not Purpur evidence. For external endpoints, "
                 "a protocol status response does not authenticate Purpur build "
                 "identity or equalize server.properties/plugins. An apparent "
                 "field match is only observed status-field equivalence."),
    }
    result = json.dumps(record, indent=2, ensure_ascii=False)
    if args.output:
        args.output.write_text(result + "\n")
    print(result)
    return 0 if comparison["unapproved_differences"] == 0 else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, EOFError, ValueError, json.JSONDecodeError) as err:
        print(f"status probe failed: {err}", file=sys.stderr)
        sys.exit(2)
