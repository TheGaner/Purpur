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
EVENT_INACTIVE = 4

WAT = """
(component
  (type $request (func (param "value" u32) (result u32)))
  (import "submit" (func $submit (type $request)))
  (core module $guest
    (import "host" "submit" (func $submit (param i32) (result i32)))
    (func (export "emit") (param i32) (result i32)
      local.get 0
      call $submit)
    (func (export "emit-then-trap") (param i32)
      local.get 0
      call $submit
      drop
      unreachable)
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
  (type $trap-export (func (param "value" u32)))
  (func (export "emit-then-trap") (type $trap-export)
    (canon lift (core func $guest-instance "emit-then-trap")))
)
"""


class HostPolicy:
    """Test-only single-thread staged admissions; no IO while guest runs."""

    def __init__(self):
        self.pending = deque()
        self.staged = []
        self.processed = []
        self.event_remaining = 0
        self.event_active = False

    def begin_event(self):
        if self.event_active or self.staged:
            raise RuntimeError("cannot begin a second event before commit/abort")
        self.event_active = True
        self.event_remaining = MAX_PER_EVENT

    def commit_event(self):
        if not self.event_active:
            raise RuntimeError("cannot commit a closed event")
        assert len(self.pending) + len(self.staged) <= MAX_PENDING
        self.pending.extend(self.staged)
        self.staged.clear()
        self.event_active = False
        self.event_remaining = 0

    def abort_event(self):
        if not self.event_active:
            raise RuntimeError("cannot abort a closed event")
        self.staged.clear()
        self.event_active = False
        self.event_remaining = 0

    def submit(self, _store, value):
        # The entire event is provisional until commit_event. Successful
        # admission does NOT authorize applying any side effect yet.
        if not self.event_active:
            return EVENT_INACTIVE
        # Placeholder policy: numeric IDs 1..10 are test-authorized. This
        # is NOT permission evaluation, identity validation or a player API.
        if not 1 <= value <= 10:
            return INVALID
        if self.event_remaining <= 0:
            return EVENT_LIMIT
        if len(self.pending) + len(self.staged) >= MAX_PENDING:
            return QUEUE_FULL
        self.staged.append(value)
        self.event_remaining -= 1
        return ADMITTED

    def drain(self, slow=False):
        if self.event_active:
            raise RuntimeError("cannot apply effects while a guest event runs")
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

    assert emit(store, 1) == EVENT_INACTIVE
    assert not policy.pending and not policy.staged
    print("PASS: calls outside an active guest event are rejected")

    policy.begin_event()
    assert emit(store, 1) == ADMITTED
    assert emit(store, 0) == INVALID
    assert list(policy.staged) == [1] and not policy.pending
    assert policy.event_remaining == 1
    print("PASS: invalid placeholder request rejected without enqueue or budget loss")
    assert emit(store, 2) == ADMITTED
    assert emit(store, 3) == EVENT_LIMIT
    assert list(policy.staged) == [1, 2] and policy.event_remaining == 0
    assert not policy.pending
    policy.commit_event()
    assert list(policy.pending) == [1, 2]
    print("PASS: two-request event budget and success-only event commit")

    policy.begin_event()
    assert emit(store, 3) == ADMITTED
    assert emit(store, 4) == ADMITTED
    assert list(policy.staged) == [3, 4]
    assert list(policy.pending) == [1, 2]
    policy.commit_event()
    assert list(policy.pending) == [1, 2, 3, 4]
    print("PASS: accepted and committed requests reach FIFO capacity four")

    policy.begin_event()
    assert emit(store, 5) == QUEUE_FULL
    assert policy.event_remaining == MAX_PER_EVENT
    assert list(policy.pending) == [1, 2, 3, 4] and not policy.staged
    policy.commit_event()
    print("PASS: saturated global FIFO refuses new request transactionally")

    assert policy.drain() == [1, 2]
    assert list(policy.pending) == [3, 4]
    assert policy.processed == [1, 2]
    print("PASS: consumer drains at most two requests, preserving FIFO backlog")

    policy.begin_event()
    assert emit(store, 5) == ADMITTED
    assert emit(store, 6) == ADMITTED
    assert emit(store, 7) == EVENT_LIMIT
    assert list(policy.staged) == [5, 6]
    assert list(policy.pending) == [3, 4]
    policy.commit_event()
    assert list(policy.pending) == [3, 4, 5, 6]
    assert policy.drain() == [3, 4]
    assert policy.drain() == [5, 6]
    assert list(policy.processed) == [1, 2, 3, 4, 5, 6]
    print("PASS: restored capacity and repeat draining preserve exact enqueue order")

    policy.begin_event()
    started = time.monotonic()
    assert burst(store, 7) == EVENT_LIMIT
    elapsed = time.monotonic() - started
    assert list(policy.staged) == [7, 7] and not policy.pending
    assert len(policy.processed) == 6  # no side effect in guest callback
    policy.commit_event()
    assert list(policy.pending) == [7, 7]
    drained = policy.drain(slow=True)
    assert drained == [7, 7] and len(policy.processed) == 8
    print(f"PASS: burst callback admitted two with no applied side effects; "
          f"callback local observed {elapsed:.4f}s, slow consumer separate "
          "(not a real-time performance guarantee)")

    policy.begin_event()
    assert emit(store, 4294967295) == INVALID
    assert not policy.pending and not policy.staged
    assert policy.event_remaining == MAX_PER_EVENT
    policy.commit_event()
    print("PASS: arbitrary guest-supplied u32 cannot bypass placeholder host policy")

    # Regression: a guest may submit an effect then trap. No host event
    # that failed should leak effects into the executable queue.
    policy.begin_event()
    fault = instance.get_func(store, "emit-then-trap")
    assert fault is not None
    try:
        fault(store, 7)
    except (wasmtime.WasmtimeError, wasmtime.Trap):
        pass
    else:
        raise AssertionError("expected guest unreachable trap")
    assert not policy.pending and list(policy.staged) == [7]
    policy.abort_event()
    assert not policy.pending and not policy.staged
    print("PASS: failed guest event's accepted requests discarded at abort")

    # A Component Model instance is invalid after trapping. Re-instantiate
    # with a new Store before simulating the next independent guest event.
    store = wasmtime.Store(engine)
    store.set_fuel(100000)
    instance = linker.instantiate(store, guest)
    emit = instance.get_func(store, "emit")
    fault = instance.get_func(store, "emit-then-trap")
    assert emit is not None and fault is not None

    # Previously committed effects survive a distinct failing callback;
    # clearing the global queue on fault would lose unrelated valid work.
    policy.begin_event()
    assert emit(store, 9) == ADMITTED
    policy.commit_event()
    assert list(policy.pending) == [9]
    policy.begin_event()
    try:
        fault(store, 7)
    except (wasmtime.WasmtimeError, wasmtime.Trap):
        pass
    else:
        raise AssertionError("expected guest trap with prior backlog")
    assert list(policy.staged) == [7] and list(policy.pending) == [9]
    policy.abort_event()
    assert list(policy.pending) == [9] and not policy.staged
    assert policy.drain() == [9]
    print("PASS: abort preserves previously committed global FIFO items")

    # A second trapped instance is also unusable; explicitly create a
    # fresh Store and guest rather than re-entering the faulted component.
    store = wasmtime.Store(engine)
    store.set_fuel(100000)
    instance = linker.instantiate(store, guest)
    emit = instance.get_func(store, "emit")
    assert emit is not None
    # Only the host-owned mock policy and isolated fixture recover here.
    policy.begin_event()
    assert emit(store, 8) == ADMITTED
    policy.commit_event()
    assert policy.drain() == [8]
    assert not policy.pending and not policy.staged
    print("PASS: fresh host event recovers capacity without stale requests")
    print("RESULT: review-only bounded admission, commit/abort, explicit "
          "rejection and FIFO verified. NOT an authenticated/owner-affine API, "
          "concurrency/timeout isolation, or actual Rust server integration.")


if __name__ == "__main__":
    main()
