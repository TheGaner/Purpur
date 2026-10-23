# Networking optimization checkpoint — October 23, 2026

## Scope

The standalone Rust server now validates the first Minecraft 26.3 / protocol-777 login-hello packet and explicitly disconnects (mismatched protocol versions are rejected without waiting for hello); it does not authenticate players or run a playable Minecraft world. This milestone benchmarks the **status TCP exchange**, not gameplay/TPS.

## Measured localhost benchmark

Reproduce using: python3 scripts/bench_status.py target/release/purpur-horizon

Rust 1.90 release build; local TCP; 10 warmups; 120 sequential status requests; 240 requests with 8 Python threads. Same host and script, background builds on the host. This is one pre-optimization run and two post-optimization runs, without statistical confidence intervals. Connection, request, response and packet parsing are included.

| Version | Sequential p50 | Sequential p95 | Concurrent p50 | Concurrent p95 | Concurrent p99 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before: socket accept in 50 ms game tick | 49.996 ms | 50.070 ms | 49.993 ms | 50.506 ms | 50.652 ms |
| After: dedicated accept thread + preframed status | 0.117 ms | 0.175 ms | 1.030 ms | 1.522 ms | 7.199 ms |
| After state-specific frame caps, repeated | 0.128 ms | 0.168 ms | 1.109 ms | 1.750 ms | 7.798 ms |

**Conclusion supported by measurements:** decoupling network acceptance from the 20Hz game tick removed the approximately 50ms status response delay on this localhost workload. Does not establish better in-game TPS, overall CPU efficiency, throughput under saturation, or packet-batching benefit.

## Implemented architecture

- src/main.rs: dedicated blocking TCP acceptor thread; active connections bounded by atomic permits (--max-connections, default 64, range 1–1024). The 20Hz game loop no longer accepts sockets.
- src/main.rs: immutable status response is JSON-encoded and fully framed once at startup, then shared as an Arc of bytes. The displayed online player count remains static zero until play sessions exist.
- src/protocol.rs: frame_read_limited rejects advertised lengths before allocating payloads. Explicit limits: handshake 1024 bytes, login hello 64 bytes, status request 1 byte, ping 9 bytes.
- src/network.rs: protocol-777 handshake status/login/transfer intent, login hello with printable 1–16 byte username and fixed 16-byte **untrusted** claimed UUID, precise trailing-data checking and login-disconnect framing. Hostname length matches Horizon StrN<255> semantics (up to 765 UTF-8 bytes, 255 Unicode characters).
- Tests: malformed and oversized VarInts and frames, hostile login names/lengths, slot admission/concurrency, on-wire preauth rejection, preframed status equivalence and status/ping/login-disconnect sockets, plus incompatible-version fast-reject on a real TCP connection.

## Incorporating prior reference research, carefully

- Horizon (MIT): compared its current protocol 777 packet definitions and LAN login code to verify packet ID, UUID field and hostname encoding. The resulting server code is an original Rust implementation, not copied Horizon runtime code.
- SteelMC (AGPL): inspired separating bounded network/command ingress from tick-critical simulation and scheduling; no SteelMC source copied here.
- FerrumC (MIT): the separation of Rust network/protocol and world data suggests an actor-based next phase; no FerrumC source imported.
- Folia (GPLv3 patch family): owner-affine world mutations still need region scheduling and handoffs; accepting TCP off-thread is safe only because it does not mutate a world.
- Krypton: reactor/event-loop, buffer pooling and compression improvements require comparison against actual real-player workloads; not yet applied or benchmarked.
- PulseNet review at /home/pc/Projects/horizon-parity-performance/pulsenet-documentation/docs/04_OPTIMIZATIONS_AND_METRICS.md: batching packet writes and coalescing can add queue allocations and cross-type ordering risks; reported savings include models rather than measured syscall reductions. The source is labeled All Rights Reserved in the local review; no PulseNet implementation copied. We deliberately do not add batching to tiny status/login messages.

## Constraints and remaining risks

1. One OS thread per active unauthenticated client remains (bounded by 64 by default). It is not appropriate for very high concurrent connections. Move to a bounded nonblocking reactor only after real play states and matched measurements justify the migration.
2. Workers have a per-read socket timeout but not an overall handshake deadline or per-peer byte/rate quotas; add cancellation and connection telemetry.
3. Listener thread currently terminates with the process, not coordinated graceful shutdown.
4. Client-declared UUID and spoofable brand do not establish authenticated Horizon identity. Version 777 is shared with compatible generic clients. Login encryption, Mojang authentication, capability negotiation, configuration, play and chunks are still absent.
5. Status reply is immutable and uses zero online players. Once live player counts exist, use a versioned snapshot refreshed after admission/leave.
6. Local measurements can be affected by parallel builds and jitter; use repeated comparisons with confidence bounds to substantiate further claims.
7. Imported upstream implementation code remains zero; this milestone inherits the original first-party AGPL-3.0-or-later license.

## Roadmap next actions

NET-03: finish bounded preauth quotas, global deadlines, telemetry, cancellation and fuzzing; accept correct protocol-777 flow.

P-01 (critical): authenticated real Horizon login, compression/encryption/configuration packets, then minimal world/session join and an agreed Horizon-specific capability protocol.

PLAY-01: future network handlers submit owned, typed actions to the simulation/region owner rather than mutating game state from TCP worker threads.

BENCH-02: after playable traffic, profile packet encoding/allocation, flush count, queue depth, p50/p95/p99, tick rate, memory and parity. Evaluate Krypton/PulseNet-type transport optimizations only if they measurably help under correct packet ordering and no unsafe source reuse.

W-01: integrate pinned SteelMC world-generation behavior with version-aligned assets and targeted golden chunks; optimize deterministic output with bounded workers only after a baseline.

## Verification evidence

All 48 Rust tests pass at this checkpoint; rustfmt and release build passed. The prototype's original native permissions and utilities tests remain included. No actual Horizon play connection, production traffic, broad fuzz corpus, world-generation parity or live player TPS test was executed.

### 2026-10-23 follow-up: slow-drip total read deadline

A shared absolute three-second deadline now bounds every preauth read,
including partial VarInts, handshake, login hello, status and optional ping.
Previous per-read SO_RCVTIMEO could be refreshed indefinitely by a peer
transmitting just ahead of each timeout. Rust source comparison: FerrumC's
MAX_HANDSHAKE_TIMEOUT wraps the handshake Future via Tokio; Horizon's
connection.rs exposes timeout-aware packet transport but a socket read
timeout alone is not necessarily an overall admission deadline. The Rust
implementation uses a minimal Read adapter and retains the existing
bounded worker pool. Added slow-drip and expired-deadline regressions.

Two post-fix localhost status-only samples: sequential p50 0.346ms/0.129ms,
8-way concurrent p95 2.062ms/1.787ms. Scheduling jitter is significant;
do not claim a new performance win. Total write deadlines, cancellation
and reactor/load benchmarks remain deferred.


### 2026-10-23 optional Ping slot-residency follow-up

A protocol-777 status-only peer previously kept the TCP worker alive around
3.035s after the handshake even though status had already been returned.
A new one-second Ping grace (capped by the pre-existing global deadline)
reduced the observed localhost worker occupancy to around 1.02s while the
original immediate Ping/Pong and a new 180ms delayed Ping test still pass.
This is a single connection timing comparison, not a controlled packet-rate
benchmark. Extremely high latency clients may not get Pong; per-IP limits,
reactor transport and full login/play remain outstanding.


### 2026-10-23 outgoing admission write-deadline follow-up

The Rust pre-authentication worker's absolute three-second deadline is now
used by both incoming reads and all outgoing response writes. Previously
SO_SNDTIMEO was fixed to three seconds per write, so a stalled outgoing
write could consume time independently of the incoming budget. The
DeadlineWriter recomputes the remaining monotonic budget before every write
and refuses an already-expired deadline. The pre-existing 1s optional
status/Ping grace also caps Pong writes. A real local stalled TCP receiver
and a 16MiB attempted send hit the 250ms deadline; ordinary status and
login framing tests still pass. This demonstrates a bounded local send,
NOT a comprehensive malicious-load benchmark or full worker cancellation.


### 2026-10-23 per-peer handshake permit fairness follow-up

The 64-worker default global permit bound did not stop a single TCP source
IP from holding all worker slots with incomplete handshakes. A new bounded
per-peer concurrent allowance (default 8, --max-per-ip) uses the actual
accepted peer address, canonicalizes IPv4-mapped IPv6, and releases both
global and source counters together on worker completion. The live TCP
fixture on Linux loopback exercised simultaneous 127.0.0.1 saturation,
independent 127.0.0.2 response and recovery. The Rust contention test uses
16 threads and verifies a per-peer cap under parallel admission. This is
not a packet-rate throttle, request-throughput benchmark, proxy-IP
authentication, or high-scale DDoS prevention; all shared-NAT users still
consume one configured IP allowance.
