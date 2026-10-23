#!/usr/bin/env python3
"""Verify native permission and utility stores permit only one live server writer.

Requires: cargo +1.90.0 build --release
Tests real processes and restarts; this is NOT a power-loss/fsync simulation.
"""
from pathlib import Path
import subprocess
import tempfile
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
            "permissions check alice test.review\nhomes alice\nstop\n",
            text=True,capture_output=True,cwd=root,timeout=8)
        assert third.returncode==0,(third.stdout,third.stderr)
        assert "Allow" in third.stdout and "Deny" in third.stdout and "base" in third.stdout, third.stdout
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
            "permissions check alice test.review\nhomes alice\nstop\n",
            text=True,capture_output=True,cwd=root,timeout=8)
        assert resumed.returncode==0,(resumed.stdout,resumed.stderr)
        assert "Allow" in resumed.stdout and "Deny" in resumed.stdout and "base" in resumed.stdout
        print("PASS: second writer denied; clean/unclean restarts preserve data")
if __name__=="__main__":
    test()
