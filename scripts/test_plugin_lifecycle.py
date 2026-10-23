#!/usr/bin/env python3
"""Verify native manifest v2 and bounded optional Java bridge shutdown.

Requires a built release binary and hello-horizon example, Java 21 JDK and jar.
No network gameplay or real Bukkit JavaPlugin API is exercised.
"""
from pathlib import Path
import subprocess
import tempfile
import time

ROOT=Path(__file__).resolve().parents[1]
SERVER=ROOT/"target/release/purpur-horizon"
PLUGIN=ROOT/"examples/rust-plugin/target/release/libhello_horizon.so"

def run(source:str, expected:str, *, hung:bool=False):
    with tempfile.TemporaryDirectory(prefix="ph-abi-review-") as dirname:
        root=Path(dirname)
        native=root/"plugins/rust"; native.mkdir(parents=True)
        java=root/"plugins/java";java.mkdir(parents=True)
        (native/"libhello_horizon.so").write_bytes(PLUGIN.read_bytes())
        (java/"Demo.java").write_text(source)
        (java/"plugin.yml").write_text("name: Demo\nmain: Demo\nversion: 1.0\n")
        subprocess.run(["javac","Demo.java"],cwd=java,check=True,capture_output=True)
        subprocess.run(["jar","cf","demo.jar","Demo.class","plugin.yml"],cwd=java,check=True,capture_output=True)
        before=time.monotonic()
        process=subprocess.Popen([str(SERVER),"--bind","127.0.0.1:0",
            "--plugins",str(root/"plugins"),"--data",str(root/"data")],
            stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,cwd=root)
        # Give the Java source launcher time to initialize before quitting
        # so the normal lifecycle test checks onDisable, not startup interruption.
        time.sleep(2.25)
        stdout,stderr=process.communicate("hi reviewed\nstop\n",timeout=8)
        elapsed=time.monotonic()-before
        assert process.returncode==0,(stdout,stderr)
        assert "[hello-horizon] command: hello reviewed" in stdout,(stdout,stderr)
        assert expected in stdout,(stdout,stderr)
        if hung:
            assert "bridge shutdown timed out; terminating child" in stderr,(stdout,stderr)
            assert elapsed<7,(elapsed,stdout,stderr)
        else:
            assert "JAVA_DISABLE_OK" in stdout,(stdout,stderr)
            assert "shutdown timed out" not in stderr,(stdout,stderr)
        print(("HUNG" if hung else "NORMAL")+" lifecycle: PASS elapsed="+str(round(elapsed,3))+"s")
        return stdout,stderr

def main():
    assert SERVER.exists() and PLUGIN.exists(), "build both release binaries first"
    run("""public class Demo {
      public void onEnable(){System.out.println("JAVA_ENABLE_OK");}
      public void onDisable(){System.out.println("JAVA_DISABLE_OK");}
    }""","JAVA_ENABLE_OK")
    run("""public class Demo {
      public void onEnable(){System.out.println("JAVA_ENABLE_OK");}
      public void onDisable(){
        System.out.println("JAVA_DISABLE_STARTED");
        try { Thread.sleep(20_000); } catch(InterruptedException ignored) {}
      }
    }""","JAVA_DISABLE_STARTED",hung=True)

if __name__=="__main__":
    main()
