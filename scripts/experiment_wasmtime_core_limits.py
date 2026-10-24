#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Optional, research-only Wasmtime 50.0.1 *core Wasm* runtime proof.

No Cargo dependency or production plugin runtime integration. Example:
  scratch=$(mktemp -d)
  python3 -m pip install --no-deps --target "$scratch" wasmtime==50.0.1
  PYTHONPATH="$scratch" python3 -B scripts/experiment_wasmtime_core_limits.py
Delete only your own scratch directory afterward.

This is NOT a WIT/component-model server plugin or a security certification.
"""
import importlib.metadata
import time

import wasmtime

WAT = """
(module
  (import "host" "note" (func $note (param i32)))
  (memory 1 2)
  (func (export "normal") (result i32)
    (i32.const 7)
    (call $note)
    (i32.const 17))
  (func (export "forever")
    (loop $again
      (br $again)))
  (func (export "grow_excessively") (result i32)
    (memory.grow (i32.const 4)))
  (func (export "blocking_host_call") (result i32)
    (i32.const 99)
    (call $note)
    (i32.const 13))
)
"""


def main():
    assert importlib.metadata.version("wasmtime") == "50.0.1", (
        "This reproducibility fixture uses the exact reviewed Python binding")
    config = wasmtime.Config()
    config.consume_fuel = True
    engine = wasmtime.Engine(config)
    module = wasmtime.Module(engine, WAT)
    assert len(module.imports) == 1

    store = wasmtime.Store(engine)
    store.set_fuel(10_000)
    calls = []

    def note(value):
        calls.append(value)
        if value == 99:
            # Host work happens outside Wasm instruction accounting.
            time.sleep(0.15)

    callback = wasmtime.Func(
        store, wasmtime.FuncType([wasmtime.ValType.i32()], []), note)
    instance = wasmtime.Instance(store, module, [callback])
    exports = instance.exports(store)

    assert exports["normal"](store) == 17
    assert calls == [7]
    print("PASS: explicitly imported host note called with 7; guest returned 17")

    store.set_fuel(500)
    started = time.monotonic()
    try:
        exports["forever"](store)
    except wasmtime.Trap as exc:
        assert "fuel" in str(exc).lower(), str(exc)
    else:
        raise AssertionError("infinite guest loop was not interrupted by fuel")
    print(f"PASS: infinite guest loop exhausted 500 fuel and trapped "
          f"({time.monotonic()-started:.4f}s local observation)")

    store.set_fuel(1000)
    assert exports["grow_excessively"](store) == -1, (
        "memory.grow past WAT's two-page maximum must fail")
    print("PASS: memory.grow 4 pages was refused by the declared 2-page max")

    missing = wasmtime.Module(
        engine,
        '(module (import "wasi_snapshot_preview1" "fd_write" (func)))')
    assert len(missing.imports) == 1
    try:
        wasmtime.Instance(store, missing, [])
    except wasmtime.WasmtimeError:
        pass
    else:
        raise AssertionError("undeclared WASI import instantiated without link")
    print("PASS: unknown/unlinked WASI import was rejected")

    store.set_fuel(200)
    started = time.monotonic()
    assert exports["blocking_host_call"](store) == 13
    elapsed = time.monotonic() - started
    assert elapsed >= 0.14, elapsed
    assert store.get_fuel() > 0
    assert calls[-1] == 99
    print(f"PASS: slow host callback took {elapsed:.3f}s without exhausting "
          "200 guest fuel (guest fuel CANNOT bound blocking host imports)")

    store.set_fuel(10_000)
    assert exports["normal"](store) == 17
    print("PASS: independent later guest callback still executes normally")
    print("RESULT: only core-module fuel/import/memory and host-call behaviors "
          "demonstrated; NOT server WIT/component model or integrated sandbox.")


if __name__ == "__main__":
    main()
