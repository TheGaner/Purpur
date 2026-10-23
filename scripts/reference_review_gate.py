#!/usr/bin/env python3
"""Audit ledger completeness and (optionally) block new feature milestones.

Usage:
  python3 scripts/reference_review_gate.py --check
  python3 scripts/reference_review_gate.py --enforce

--check validates inventory coverage and current source digests for reviews
already in progress. It NEVER certifies correctness, parity, or licensing.
--enforce requires evidence-backed approval and current file digests for every
retrospective review item. Until that happens it MUST fail intentionally.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs/IMPLEMENTATION_REVIEW_STATUS.json"
VALID = {"pending", "in_review", "approved"}

def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(enforce: bool) -> int:
    problems = []
    try:
        ledger = json.loads(MANIFEST.read_text())
    except (OSError, ValueError) as exc:
        print(f"Cannot load audit ledger: {exc}", file=sys.stderr)
        return 2

    if ledger.get("version") != 1:
        problems.append("Unsupported or missing ledger schema version")
    entries = ledger.get("reviews")
    if not isinstance(entries, list) or not entries:
        print("No review entries found", file=sys.stderr)
        return 2

    known = set()
    covered = set()
    blocked = []
    for entry in entries:
        name = entry.get("id")
        if not isinstance(name, str) or not name.startswith("REV-") or name in known:
            problems.append(f"Invalid or duplicate review ID: {name!r}")
            continue
        known.add(name)
        status = entry.get("status")
        if status not in VALID:
            problems.append(f"{name}: invalid status {status!r}")
        paths = entry.get("paths", [])
        refs = entry.get("references", [])
        evidence = entry.get("evidence")
        if not paths or not isinstance(paths, list) or not refs or not isinstance(refs, list):
            problems.append(f"{name}: missing source-path or upstream reference inventory")
            continue
        if not isinstance(evidence, str) or not (ROOT / evidence).is_file():
            problems.append(f"{name}: missing evidence document")
        hashes = entry.get("approved_sha256", {})
        if not isinstance(hashes, dict):
            problems.append(f"{name}: approved_sha256 must be a map")
            hashes = {}
        current = entry.get("current_sha256", {})
        if status in ("in_review", "approved") and not isinstance(current, dict):
            problems.append(f"{name}: current_sha256 must be a map")
            current = {}
        if status in ("in_review", "approved"):
            extras = sorted(set(current) - set(paths))
            if extras:
                problems.append(f"{name}: hashes for unowned paths: {extras}")
        for path in paths:
            if not isinstance(path, str) or path.startswith("/") or ".." in Path(path).parts:
                problems.append(f"{name}: invalid or unsafe path {path!r}")
                continue
            f = ROOT / path
            if not f.is_file():
                problems.append(f"{name}: missing file {path}")
            covered.add(path)
            if f.is_file() and status in ("in_review", "approved"):
                if current.get(path) != sha256(f):
                    problems.append(f"{name}: missing/stale current sha256 for {path}")
            if f.is_file() and status == "approved" and hashes.get(path) != sha256(f):
                problems.append(f"{name}: missing/stale approved sha256 for {path}")
        if status == "approved":
            for prop in ("upstream_code_refs", "alternatives", "test_evidence", "decision"):
                value = entry.get(prop)
                if not isinstance(value, str) or len(value.strip()) < 20:
                    problems.append(f"{name}: approved without substantial {prop}")
        elif enforce:
            blocked.append(f"{name}={status}")

    tracked = subprocess.run(
        ["git", "ls-files", "--", "src", "java", "scripts"],
        cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.splitlines()
    # Audit test/compliance Python source alongside Rust and Java runtime
    # source; untracked caches are deliberately outside the code inventory.
    source_files = {x for x in tracked if x.endswith((".rs", ".java", ".py"))}
    missing = sorted(source_files - covered)
    if missing:
        problems.append(f"Source files without retrospective review owner: {', '.join(missing)}")

    print(f"Reference review ledger: {len(entries)} groups, "
          f"{len(source_files)} tracked source files, {len(missing)} uncovered")
    print(f"Approval status: {sum(e.get('status')=='approved' for e in entries)} "
          f"approved / {len(entries)} total")
    for problem in problems:
        print(f"ERROR: {problem}", file=sys.stderr)
    if enforce and blocked:
        print("REVIEW HOLD: " + ", ".join(blocked), file=sys.stderr)
    if problems or (enforce and blocked):
        print("Gate not cleared; no new feature milestone is authorized.", file=sys.stderr)
        return 1
    print("Coverage/check passed" if not enforce else "All source reviews approved and current")
    return 0

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--enforce", action="store_true")
    args = parser.parse_args()
    sys.exit(run(args.enforce))
