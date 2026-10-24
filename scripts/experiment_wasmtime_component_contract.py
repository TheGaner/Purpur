#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review-only typed WebAssembly Component Model host/guest proof.

Requires wasmtime-py==50.0.1 on PYTHONPATH. Dependency must be installed
temporarily with pip --target and removed afterward. No production server
or Rust dependency graph is changed. The WIT-shaped host contract is
documented in fixtures/server-plugin-probe.wit; this test hand-assembles
its component WAT equivalent. This is NOT WIT bindgen code generation.
"""
from pathlib import Path
import importlib.metadata
import time

import wasmtime
import wasmtime.component as component

WIT = Path(__file__).parent / "fixtures" / "server-plugin-probe.wit"

COMPONENT_WAT = """
(component
  (type $note-type (func (param "value" u32)))
  (import "note" (func $note (type $note-type)))
  (core module $guest
    (import "host" "note" (func $note (param i32)))
    (func (export "run") (param i32) (result i32)
      (local.get 0)
      (call $note)
      (local.get 0)
      (i32.const 10)
      (i32.add))
    (func (export "forever")
      (loop $spin (br $spin)))
  )
  (core func $lowered-note (canon lower (func $note)))
  (core instance $instance (instantiate $guest
    (with "host" (instance
      (export "note" (func $lowered-note))))))
  (type $run-type (func (param "value" u32) (result u32)))
  (func (export "run") (type $run-type)
    (canon lift (core func $instance "run")))
  (func (export "forever")
    (canon lift (core func $instance "forever")))
)
"""


def make_linker(engine, callback=None):
    linker = component.Linker(engine)
    if callback is not None:
        with linker.root() as root:
            # Component callbacks take store context before typed args.
            root.add_func("note", callback)
    return linker


def main():
    assert importlib.metadata.version("wasmtime") == "50.0.1", (
        "Use only reviewed wasmtime-py 50.0.1")
    wit = WIT.read_text()
    assert "world server-probe" in wit and "import note: func(value: u32)" in wit
    assert "export run: func(value: u32) -> u32" in wit
    print("PASS: review-only WIT declaration describes typed host/event API")

    conf = wasmtime.Config()
    conf.consume_fuel = True
    engine = wasmtime.Engine(conf)
    guest = component.Component(engine, COMPONENT_WAT)
    print("PASS: actual Component Model text compiles with canonical lowering/lifting")

    rejected = False
    isolated_store = wasmtime.Store(engine)
    isolated_store.set_fuel(10_000)
    try:
        make_linker(engine).instantiate(isolated_store, guest)
    except wasmtime.WasmtimeError:
        rejected = True
    assert rejected, "component must reject missing host note capability"
    print("PASS: missing host capability refuses guest instantiation")

    received = []

    def host_note(store, value):
        assert store is not None
        received.append(value)
        if value == 99:
            time.sleep(0.15)

    store = wasmtime.Store(engine)
    store.set_fuel(10_000)
    instance = make_linker(engine, host_note).instantiate(store, guest)
    run = instance.get_func(store, "run")
    assert run is not None and run(store, 7) == 17
    assert received == [7]
    print("PASS: typed u32 component import/export and guest return agree")

    try:
        run(store, "forged-string")
    except (TypeError, ValueError, OverflowError):
        pass
    else:
        raise AssertionError("wrong typed argument unexpectedly accepted")
    print("PASS: wrong typed host->guest argument rejected")

    store.set_fuel(200)
    start = time.monotonic()
    assert run(store, 99) == 109
    elapsed = time.monotonic() - start
    assert elapsed >= 0.14 and store.get_fuel() > 0 and received[-1] == 99
    print(f"PASS: blocking component host import took {elapsed:.3f}s with "
          "guest fuel remaining; host code needs independent deadline")

    forever = instance.get_func(store, "forever")
    assert forever is not None
    store.set_fuel(500)
    try:
        forever(store)
    except (wasmtime.Trap, wasmtime.WasmtimeError) as exc:
        # v50 component invocation wraps the core Wasm fuel trap in
        # WasmtimeError, unlike some core-module Wasm APIs.
        assert "fuel" in str(exc).lower(), str(exc)
    else:
        raise AssertionError("infinite component guest did not trap at fuel limit")
    print("PASS: infinite loop inside component trapped at 500 guest fuel")

    fresh_store = wasmtime.Store(engine)
    fresh_store.set_fuel(10_000)
    fresh = make_linker(engine, host_note).instantiate(fresh_store, guest)
    fresh_fn = fresh.get_func(fresh_store, "run")
    assert fresh_fn is not None and fresh_fn(fresh_store, 5) == 15
    print("PASS: fresh independently instantiated component remains usable")

    print("RESULT: genuine Component Model canonical import/export tested; "
          "this is NOT WIT-bindgen generated, OS-sandboxed, or integrated "
          "into Purpur-Horizon's server/plugin lifecycle.")


if __name__ == "__main__":
    main()
