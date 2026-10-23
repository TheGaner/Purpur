#!/usr/bin/env python3
"""Local protocol-777 status TCP latency benchmark (not a gameplay/TPS benchmark).

Run with: python3 scripts/bench_status.py [path/to/purpur-horizon]
Other CPU-intensive builds can affect results. Compare on same hardware/load.
"""
import json, socket, subprocess, sys, tempfile, time, statistics
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

BINARY = str(Path(sys.argv[1] if len(sys.argv)>1 else "target/release/purpur-horizon").resolve())
def vi(number):
    result=bytearray()
    while True:
        if number<128:return bytes(result)+bytes([number])
        result.append((number&127)|128);number>>=7

def frame(body):
    return vi(len(body))+body

def read_varint(conn):
    result=0
    for shift in range(0,35,7):
        raw=conn.recv(1)
        if not raw:raise EOFError("unexpected EOF")
        result|=(raw[0]&127)<<shift
        if raw[0]&128==0:return result
    raise ValueError("VarInt overflow")

def exactly(sock,n):
    buf=bytearray()
    while len(buf)<n:
        next=sock.recv(n-len(buf))
        if not next:raise EOFError("truncated response")
        buf.extend(next)
    return bytes(buf)

def sample(port):
    start=time.perf_counter_ns()
    with socket.create_connection(("127.0.0.1",port),timeout=3) as conn:
        conn.settimeout(3)
        packet=bytearray(b'\0')
        packet+=vi(777)+vi(9)+b'localhost'+bytes((0x63,0xdd,1))
        conn.sendall(frame(packet)+frame(b'\0'))
        length=read_varint(conn)
        result=exactly(conn,length)
        if not result or result[0]!=0 or b'"protocol":777' not in result:
            raise ValueError("unexpected status response")
    return (time.perf_counter_ns()-start)/1e6

def stats(values):
    values=sorted(values)
    return {"count":len(values),"p50_ms":round(statistics.median(values),3),
            "p95_ms":round(values[int(.95*(len(values)-1))],3),
            "p99_ms":round(values[int(.99*(len(values)-1))],3),
            "mean_ms":round(statistics.mean(values),3)}

def run():
    with tempfile.TemporaryDirectory(prefix="ph-status-bench-") as d:
        with socket.socket() as s:
            s.bind(("127.0.0.1",0))
            port=s.getsockname()[1]
        with open(Path(d)/"server.log","w+") as log:
            proc=subprocess.Popen([BINARY,"--bind",f"127.0.0.1:{port}",
                "--plugins",str(Path(d)/"plugins"),"--data",str(Path(d)/"data"),
                "--max-connections","64"],stdin=subprocess.PIPE,stdout=log,stderr=log,text=True)
            try:
                time.sleep(.16)
                if proc.poll() is not None:raise RuntimeError("server stopped during startup")
                for _ in range(10):sample(port)
                sequential=[sample(port) for _ in range(120)]
                with ThreadPoolExecutor(max_workers=8) as executor:
                    simultaneous=list(executor.map(lambda _:sample(port),range(240)))
                data={"binary":BINARY,"sequential":stats(sequential),
                      "concurrent_8":stats(simultaneous),"notes":"localhost TCP, status/ping only, no gameplay or TLS; 10 warmups"}
                print(json.dumps(data,indent=2))
                return data
            finally:
                if proc.poll() is None:
                    proc.stdin.write("stop\n");proc.stdin.flush()
                    try:proc.communicate(timeout=3)
                    except subprocess.TimeoutExpired:proc.kill();proc.wait()
                log.flush()
                if proc.returncode:print("SERVER LOG:",Path(d,"server.log").read_text()[-2000:],file=sys.stderr)
if __name__=="__main__":run()
