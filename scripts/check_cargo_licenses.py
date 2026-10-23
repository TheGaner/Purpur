#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check exact locked Cargo package checksums and declared SPDX metadata.

Only an explicitly reviewed --accept-current refresh changes the snapshot.
This is a drift check, not verification of per-file copyrights or license texts.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "docs" / "CARGO_DEPENDENCY_LICENSES.json"


def actual_inventory(root: Path = ROOT) -> dict:
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    env = os.environ.copy()
    if "RUSTUP_HOME" not in env and (root / ".rustup").is_dir():
        env["RUSTUP_HOME"] = str(root / ".rustup")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "+1.90.0", "metadata", "--locked", "--offline", "--format-version", "1"],
        cwd=root, env=env, text=True,
    ))
    registry = {}
    for pkg in metadata["packages"]:
        if pkg.get("source") is None:
            continue
        key = (pkg["name"], pkg["version"], pkg["source"])
        if key in registry:
            raise ValueError(f"ambiguous Cargo metadata package: {key}")
        registry[key] = pkg

    packages = []
    for pkg in lock["package"]:
        source = pkg.get("source")
        if source is None:
            continue
        key = (pkg["name"], pkg["version"], source)
        if key not in registry:
            raise ValueError(f"locked package missing from resolved metadata: {key}")
        resolved = registry.pop(key)
        expression = resolved.get("license")
        if not isinstance(expression, str) or not expression.strip():
            raise ValueError(f"missing declared SPDX license for {pkg['name']}")
        checksum = pkg.get("checksum")
        if not isinstance(checksum, str) or len(checksum) != 64:
            raise ValueError(f"missing or malformed locked checksum for {pkg['name']}")
        packages.append({
            "name": pkg["name"],
            "version": pkg["version"],
            "source": source,
            "checksum": checksum,
            "declaredLicense": expression,
            "repository": resolved.get("repository"),
        })
    if registry:
        raise ValueError(f"resolved packages missing from Cargo.lock: {sorted(registry)}")
    packages.sort(key=lambda p: (p["name"], p["version"], p["source"]))
    return {
        "schemaVersion": 1,
        "scope": "locked Cargo registry crates only; metadata declarations, not legal clearance",
        "packages": packages,
    }


def differences(expected: dict, current: dict) -> list[str]:
    if expected == current:
        return []
    problems = []
    if expected.get("schemaVersion") != current["schemaVersion"]:
        problems.append("schema version differs")
    if expected.get("scope") != current["scope"]:
        problems.append("scope differs")
    old = {(p["name"], p["version"], p["source"]): p for p in expected.get("packages", [])}
    new = {(p["name"], p["version"], p["source"]): p for p in current["packages"]}
    for name in sorted(old.keys() - new.keys()):
        problems.append(f"removed/updated crate: {name}")
    for name in sorted(new.keys() - old.keys()):
        problems.append(f"added/updated crate: {name}")
    for name in sorted(old.keys() & new.keys()):
        if old[name] != new[name]:
            problems.append(f"crate checksum, declared license, or metadata drift: {name}")
    if not problems:
        problems.append("snapshot structure, order, or fields differ")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--accept-current", action="store_true",
        help="replace pinned metadata snapshot AFTER separately reviewing all changed licenses",
    )
    args = parser.parse_args()
    try:
        current = actual_inventory()
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError) as exc:
        print(f"ERROR: Cargo license inventory failed: {exc}", file=sys.stderr)
        return 2
    if args.accept_current:
        SNAPSHOT.write_text(json.dumps(current, indent=2) + "\n")
        print(f"SNAPSHOT WRITTEN: {len(current['packages'])} registry crates. "
              "Review license obligations and corresponding source before committing.")
        return 0
    if not SNAPSHOT.is_file():
        print("ERROR: missing reviewed dependency snapshot; review then use --accept-current",
              file=sys.stderr)
        return 1
    try:
        expected = json.loads(SNAPSHOT.read_text())
        problems = differences(expected, current)
    except (ValueError, KeyError, TypeError) as exc:
        print(f"ERROR: malformed dependency license snapshot: {exc}", file=sys.stderr)
        return 1
    if problems:
        for problem in problems:
            print(f"LICENSE REVIEW NEEDED: {problem}", file=sys.stderr)
        return 1
    print(f"PASS: {len(current['packages'])} exact locked Cargo crates and declared "
          "licenses match reviewed snapshot (not legal certification)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
