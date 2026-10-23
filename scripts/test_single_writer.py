#!/usr/bin/env python3
"""Verify native permission and utility stores permit only one live server writer.

Requires: cargo +1.90.0 build --release
Tests real processes and restarts; this is NOT a power-loss/fsync simulation.
"""
from pathlib import Path
import subprocess
import tempfile
import json
import time

ROOT=Path(__file__).resolve().parents[1]
BINARY=ROOT/"target/release/purpur-horizon"

def test():
    with tempfile.TemporaryDirectory(prefix="purpur-json-lock-") as folder:
        root=Path(folder)
        (root/"plugins").mkdir()
        args=[str(BINARY),"--bind","127.0.0.1:0",
              "--data",str(root/"data"),"--plugins",str(root/"plugins")]
        with (root/"one.log").open("w+") as log:
            server=subprocess.Popen(args,stdin=subprocess.PIPE,
                stdout=log,stderr=log,text=True,cwd=root)
            try:
                time.sleep(0.4)
                assert server.poll() is None, "first server failed to start"
                second=subprocess.run(args,input="stop\n",text=True,
                    capture_output=True,cwd=root,timeout=6)
                assert second.returncode != 0, (second.stdout,second.stderr)
                assert "store already open by another writer" in second.stderr,second.stderr
                out,err=server.communicate(
                    "permissions set alice essentials.home allow\n"
                    "permissions set ALICE test.review deny\n"
                    "permissions set user:ALICE test.qualified allow\n"
                    "permissions set group:STAFF test.inherited deny\n"
                    "permissions parent user:ALICE group:STAFF\n"
                    "sethome alice base world 1 65 2\nstop\n",
                    timeout=8)
                assert server.returncode==0, (out,err)
                assert (root/"data"/"permissions.json").is_file()
                assert (root/"data"/"utilities.json").is_file()
            finally:
                if server.poll() is None:
                    server.kill()
                    server.wait(timeout=3)
        third=subprocess.run(args,input=
            "permissions check alice essentials.home\n"
            "permissions check alice test.review\n"
            "permissions check user:alice test.qualified\n"
            "permissions check alice test.inherited\n"
            "permissions check group:staff test.inherited\n"
            "homes alice\nstop\n",
            text=True,capture_output=True,cwd=root,timeout=8)
        assert third.returncode==0,(third.stdout,third.stderr)
        assert "Allow" in third.stdout and "Deny" in third.stdout and "base" in third.stdout, third.stdout
        assert "user:alice test.qualified: Allow" in third.stdout
        assert "alice test.inherited: Deny" in third.stdout
        assert "group:staff test.inherited: Undefined" in third.stdout
        # An unclean process exit must release the OS advisory locks.
        with (root/"crash.log").open("w+") as log:
            crashed=subprocess.Popen(args,stdin=subprocess.PIPE,
                stdout=log,stderr=log,text=True,cwd=root)
            try:
                time.sleep(0.3)
                assert crashed.poll() is None, "crash fixture failed to start"
            finally:
                if crashed.poll() is None:
                    crashed.kill()
                crashed.wait(timeout=4)
        resumed=subprocess.run(args,input=
            "permissions check alice essentials.home\n"
            "permissions check alice test.review\n"
            "permissions check user:alice test.qualified\n"
            "permissions check alice test.inherited\n"
            "permissions check group:staff test.inherited\n"
            "homes alice\nstop\n",
            text=True,capture_output=True,cwd=root,timeout=8)
        assert resumed.returncode==0,(resumed.stdout,resumed.stderr)
        assert "Allow" in resumed.stdout and "Deny" in resumed.stdout and "base" in resumed.stdout
        assert "user:alice test.qualified: Allow" in resumed.stdout
        assert "alice test.inherited: Deny" in resumed.stdout
        assert "group:staff test.inherited: Undefined" in resumed.stdout
        # Old/imported v1 JSON may use capitalized user, home and warp keys.
        # Validate real console lookups, rewrite-on-save, and ambiguous startup.
        legacy_root=root/"imported"
        (legacy_root/"plugins").mkdir(parents=True)
        (legacy_root/"data").mkdir()
        loc={"world":"World","x":1.0,"y":64.0,"z":3.0}
        legacy_file=legacy_root/"data"/"utilities.json"
        legacy_file.write_text(json.dumps({"version":1,"homes":{"Alex":{"Base":loc}},
                                          "warps":{"Spawn":loc}}))
        legacy_args=[str(BINARY),"--bind","127.0.0.1:0",
                     "--data",str(legacy_root/"data"),
                     "--plugins",str(legacy_root/"plugins")]
        converted=subprocess.run(legacy_args,input=
            "home alex base\nwarp spawn\nsetwarp lobby World 1 65 2\nstop\n",
            text=True,capture_output=True,cwd=legacy_root,timeout=8)
        assert converted.returncode==0,(converted.stdout,converted.stderr)
        assert "Home alex/base: World" in converted.stdout,converted.stdout
        assert "spawn: World" in converted.stdout,converted.stdout
        saved=json.loads(legacy_file.read_text())
        assert list(saved["homes"])==["alex"]
        assert list(saved["homes"]["alex"])==["base"]
        assert "spawn" in saved["warps"] and "Spawn" not in saved["warps"]
        saved["warps"]["Spawn"]=loc
        legacy_file.write_text(json.dumps(saved))
        conflict=subprocess.run(legacy_args,input="stop\n",text=True,
            capture_output=True,cwd=legacy_root,timeout=8)
        assert conflict.returncode!=0,(conflict.stdout,conflict.stderr)
        assert "ambiguous case-colliding" in conflict.stderr,conflict.stderr
        print("PASS: second writer denied; clean/unclean restarts, legacy utilities migration and collision rejection")
if __name__=="__main__":
    test()
