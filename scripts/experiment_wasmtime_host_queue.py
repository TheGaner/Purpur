#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review-only bounded host admission using Wasmtime Component Model 50.0.1.

No filesystem/network/world operation happens inside a guest import. All
side effects are simulated after guest return by an explicitly limited,
host-owned FIFO consumer. This is NOT a production plugin runtime,
permission system, authenticated queue, or concurrency safety proof.
Install exact wasmtime-py==50.0.1 to a throwaway pip --target directory,
set PYTHONPATH for one process, and delete the directory on exit.
"""
from collections import deque
import importlib.metadata
from pathlib import Path
import time

import wasmtime
import wasmtime.component as component


WIT_PATH = Path(__file__).parent / "fixtures" / "server-plugin-probe.wit"
MAX_PER_EVENT = 2
MAX_PENDING = 4
MAX_DRAIN_PER_TICK = 2

ADMITTED = 0
INVALID = 1
EVENT_LIMIT = 2
QUEUE_FULL = 3

WAT = """
(component
  (type $request (func (param "value" u32) (result u32)))
  (import "submit" (func $submit (type $request)))
  (core module $guest
    (import "host" "submit" (func $submit (param i32) (result i32)))
    (func (export "emit") (param i32) (result i32)
      local.get 0
      call $submit)
    (func (export "burst-three") (param i32) (result i32)
      local.get 0
      call $submit
      drop
      local.get 0
      call $submit
      drop
      local.get 0
      call $submit)
  )
  (core func $lower (canon lower (func $submit)))
  (core instance $guest-instance
    (instantiate $guest (with "host" (instance
      (export "submit" (func $lower))))))
  (type $export (func (param "value" u32) (result u32)))
  (func (export "emit") (type $export)
    (canon lift (core func $guest-instance "emit")))
  (func (export "burst-three") (type $export)
    (canon lift (core func $guest-instance "burst-three")))
)
"""


class HostPolicy:
    """Test-only single-thread admission and separate tick-side effects."""

    def __init__(self):
        self.pending = deque()
        self.processed = []
        self.event_remaining = 0

    def begin_event(self):
        self.event_remaining = MAX_PER_EVENT

    def submit(self, _store, value):
        # Placeholder policy: numeric IDs 1..10 are test-authorized. This
        # is NOT permission evaluation, identity validation or a player API.
        if not 1 <= value <= 10:
            return INVALID
        if self.event_remaining <= 0:
            return EVENT_LIMIT
        if len(self.pending) >= MAX_PENDING:
            return QUEUE_FULL
        self.pending.append(value)
        self.event_remaining -= 1
        return ADMITTED

    def drain(self, slow=False):
        completed = []
        for _ in range(min(MAX_DRAIN_PER_TICK, len(self.pending))):
            item = self.pending.popleft()
            # A real host must recheck permissions/ownership/context
            # before applying any command, and coordinate cancellation.
            if not 1 <= item <= 10:
                raise AssertionError("invalid request reached consumer")
            if slow:
                # Deliberately expensive consumer operation, AFTER the
                # component callback returned and never in host import.
                time.sleep(0.15)
            self.processed.append(item)
            completed.append(item)
        return completed


def main():
    assert importlib.metadata.version("wasmtime") == "50.0.1"
    wit = WIT_PATH.read_text()
    assert "world bounded-request-probe" in wit
    assert "import submit: func(value: u32) -> u32" in wit
    print("PASS: WIT-shaped review contract explicitly declares bounded submit result")

    config = wasmtime.Config()
    config.consume_fuel = True
    engine = wasmtime.Engine(config)
    guest = component.Component(engine, WAT)
    policy = HostPolicy()
    linker = component.Linker(engine)
    with linker.root() as root:
        root.add_func("submit", policy.submit)
    store = wasmtime.Store(engine)
    store.set_fuel(100000)
    instance = linker.instantiate(store, guest)
    emit = instance.get_func(store, "emit")
    burst = instance.get_func(store, "burst-three")
    assert emit is not None and burst is not None
    print("PASS: actual typed Component Model import/export instantiated without WASI")

    policy.begin_event()
    assert emit(store, 1) == ADMITTED
    assert emit(store, 0) == INVALID
    assert list(policy.pending) == [1] and policy.event_remaining == 1
    print("PASS: invalid placeholder request rejected without enqueue or budget loss")
    assert emit(store, 2) == ADMITTED
    assert emit(store, 3) == EVENT_LIMIT
    assert list(policy.pending) == [1, 2] and policy.event_remaining == 0
    print("PASS: two-request event budget prevents third admission")

    policy.begin_event()
    assert emit(store, 3) == ADMITTED
    assert emit(store, 4) == ADMITTED
    assert list(policy.pending) == [1, 2, 3, 4]
    print("PASS: accepted requests reach host-owned FIFO capacity four")

    policy.begin_event()
    assert emit(store, 5) == QUEUE_FULL
    assert policy.event_remaining == MAX_PER_EVENT
    assert list(policy.pending) == [1, 2, 3, 4]
    print("PASS: saturated global FIFO refuses new request transactionally")

    assert policy.drain() == [1, 2]
    assert list(policy.pending) == [3, 4]
    assert policy.processed == [1, 2]
    print("PASS: consumer drains at most two requests, preserving FIFO backlog")

    assert emit(store, 5) == ADMITTED
    assert emit(store, 6) == ADMITTED
    assert emit(store, 7) == EVENT_LIMIT
    assert list(policy.pending) == [3, 4, 5, 6]
    assert policy.drain() == [3, 4]
    assert policy.drain() == [5, 6]
    assert list(policy.processed) == [1, 2, 3, 4, 5, 6]
    print("PASS: restored capacity and repeat draining preserve exact enqueue order")

    policy.begin_event()
    started = time.monotonic()
    assert burst(store, 7) == EVENT_LIMIT
    elapsed = time.monotonic() - started
    assert list(policy.pending) == [7, 7]
    assert len(policy.processed) == 6  # no side effect in guest callback
    drained = policy.drain(slow=True)
    assert drained == [7, 7] and len(policy.processed) == 8
    print(f"PASS: burst callback admitted two with no applied side effects; "
          f"callback local observed {elapsed:.4f}s, slow consumer separate "
          "(not a real-time performance guarantee)")

    policy.begin_event()
    assert emit(store, 4294967295) == INVALID
    assert not policy.pending and policy.event_remaining == MAX_PER_EVENT
    print("PASS: arbitrary guest-supplied u32 cannot bypass placeholder host policy")
    print("RESULT: review-only bounded admission, explicit rejection and staged "
          "FIFO verified. NOT an authenticated/owner-affine safe plugin API, "
          "concurrency/timeout isolation, or actual Rust server integration.")


if __name__ == "__main__":
    main()
