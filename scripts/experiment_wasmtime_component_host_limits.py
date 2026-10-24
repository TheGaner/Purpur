#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review-only host-enforced Component Model store limits and epoch proof.

Requires wasmtime-py==50.0.1 installed temporarily on PYTHONPATH.
Reuses the first-party scalar canonical import/export WAT but adds an
UNBOUNDED guest memory for testing the independent store-side limit.
No real server or Cargo dependency is changed.
"""
import importlib.metadata
import threading
import time

import wasmtime
import wasmtime.component as component

from experiment_wasmtime_component_contract import COMPONENT_WAT, make_linker


def memory_guest_wat():
    anchor = '    (import "host" "note" (func $note (param i32)))\n'
    assert COMPONENT_WAT.count(anchor) == 1
    wasm = COMPONENT_WAT.replace(
        anchor,
        anchor + '    (memory 1)\n'
        + '    (func (export "grow") (result i32)\n'
        + '      (memory.grow (i32.const 1)))\n',
        1,
    )
    # The core memory has NO guest-declared maximum; any cap in the
    # denied case must come from the host Store.set_limits.
    export_anchor = '  (func (export "forever")\n    (canon lift'
    assert wasm.count(export_anchor) == 1
    return wasm.replace(
        export_anchor,
        '  (func (export "grow") (result s32)\n'
        + '    (canon lift (core func $instance "grow")))\n'
        + export_anchor,
        1,
    )


def instantiate(engine, wasm, store):
    guest = component.Component(engine, wasm)
    linker = make_linker(engine, lambda _store, _value: None)
    instance = linker.instantiate(store, guest)
    return instance


def main():
    assert importlib.metadata.version("wasmtime") == "50.0.1"
    text = memory_guest_wat()

    engine = wasmtime.Engine()
    bounded = wasmtime.Store(engine)
    bounded.set_limits(memory_size=65536)
    a = instantiate(engine, text, bounded)
    grow_a = a.get_func(bounded, "grow")
    assert grow_a is not None
    assert grow_a(bounded) == -1, (
        "host 64KiB limit must reject memory.grow on an unbounded guest")
    print("PASS: host Store.memory_size 64KiB blocked an otherwise unbounded "
          "component guest memory.grow")

    unbounded = wasmtime.Store(engine)
    b = instantiate(engine, text, unbounded)
    grow_b = b.get_func(unbounded, "grow")
    assert grow_b is not None
    assert grow_b(unbounded) == 1, "unlimited control must grow 1->2 pages"
    print("PASS: same guest grows memory without host Store cap (control)")

    conf = wasmtime.Config()
    conf.epoch_interruption = True
    epoch_engine = wasmtime.Engine(conf)
    epoch_store = wasmtime.Store(epoch_engine)
    epoch_store.set_epoch_deadline(1)
    c = instantiate(epoch_engine, COMPONENT_WAT, epoch_store)
    infinite = c.get_func(epoch_store, "forever")
    assert infinite is not None
    ticker = threading.Timer(0.05, epoch_engine.increment_epoch)
    ticker.daemon = True
    ticker.start()
    try:
        started = time.monotonic()
        try:
            infinite(epoch_store)
        except (wasmtime.Trap, wasmtime.WasmtimeError) as exc:
            message = str(exc).lower()
            assert ("interrupt" in message or "epoch" in message
                    or "deadline" in message), message
        else:
            raise AssertionError("external epoch did not interrupt guest")
        duration = time.monotonic() - started
        print(f"PASS: externally incremented epoch interrupted infinite "
              f"component callback (local observed {duration:.3f}s; "
              "NOT a deterministic wall-clock guarantee)")
    finally:
        ticker.join(timeout=1)

    # A separate new store with a future epoch deadline should still
    # accept a normal typed event despite the prior instance trap.
    clean = wasmtime.Store(epoch_engine)
    clean.set_epoch_deadline(1)
    d = instantiate(epoch_engine, COMPONENT_WAT, clean)
    run = d.get_func(clean, "run")
    assert run is not None and run(clean, 2) == 12
    print("PASS: fresh store and component remain usable after epoch trap")
    print("RESULT: genuine component guest Store cap + external epoch "
          "behavior verified; NO bounded blocking host import, server-side "
          "WIT binding, production isolation or plugin lifecycle integration.")


if __name__ == "__main__":
    main()
