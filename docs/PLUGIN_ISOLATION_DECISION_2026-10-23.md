# Plugin isolation — provisional architecture decision (2026-10-23)

**Research and requirements only. NO new sandbox or runtime is implemented; Phase -1 remains active.** This supports master task RV-PLUG in [ROADMAP.md](ROADMAP.md), with source comparison in [IMPLEMENTATION_REFERENCE_AUDIT.md](IMPLEMENTATION_REFERENCE_AUDIT.md).

## What exists and what fails

The current native Rust plugin host (src/native.rs and src/plugins.rs) invokes shared-library C ABI entry points synchronously inside the server address space, with complete process privileges. The host-owned v2 command manifest fixed an unbounded raw pointer scan but **does not** isolate a malicious plugin, prevent memory corruption, or stop an infinite loop. The repeatable real-server script scripts/test_native_plugin_nontermination.py confirms that a native callback can prevent shutdown until its disposable host process is forcibly killed. This is a local trusted-plugin exposure, not a demonstrated remote exploit.

The Java bridge lives in another process with a bounded child shutdown but lacks Bukkit/Paper APIs and cannot protect against native C ABI callbacks. Startup now validates and locks both JSON stores before plugin enable events (fix 533ee3b), preventing the losing second server from running startup effects; that does not isolate callbacks after a healthy boot.

## Pinned primary comparator and official mechanisms

- Horizon source commit e325fa65878071d5c9076e908e9a4adb258b3313, crates/horizon-mods/src/runtime.rs: Wasmtime component model, Config::consume_fuel(true), init/tick/event fuel (2,000,000,000 / 50,000,000 / 100,000,000), guest linear memory cap 64 MiB, instance cap 16, imported capability checks, storage/host-output quotas, failed status on traps. These are **client runtime examples**, not measured server limits.
- **Crucial gap:** the pinned Horizon runtime explicitly rejects manifest Side::Server with "server mods do not run yet". It cannot be transplanted directly as a working server plugin host or used as evidence of working server mods.
- Wasmtime official security design: https://docs.wasmtime.dev/security.html — guest WebAssembly address space and OS capabilities are mediated by explicitly linked imports. Exposing unsafe host functions can still compromise the host.
- Wasmtime official interruption explanation: https://docs.wasmtime.dev/examples-interrupting-wasm.html — deterministic fuel budgets versus less deterministic, lower-overhead epoch interrupts.
- Wasmtime Store/Config APIs: https://docs.wasmtime.dev/api/wasmtime/struct.Store.html and https://docs.wasmtime.dev/api/wasmtime/struct.Config.html — separately configured resource limits and epoch deadlines. Guest fuel does NOT bound synchronous, blocking host I/O or arbitrary native host callbacks.
- Paper/Purpur Bukkit plugin format and Folia owner-thread semantics require a separate compatibility architecture; LuckPerms/EssentialsX are service policies, FerrumC plugin crate is a placeholder and SteelMC command handling is not isolation. Networking optimizers are unrelated.
- Dependency/legal caveat: adding Wasmtime, WIT bindings, build artifacts and new crates must pass a fresh exact pinned REV-LEGAL license/source/SBOM audit; the current 11-crate snapshot is not evidence for an expanded runtime.

## Explicit alternatives and provisional selection

| Model | Memory isolation | Recover from hung callback | Legacy C ABI | Risks |
|---|---|---|---|---|
| A. Current synchronous dlopen | None | No | Yes | Already fails disposable hang test |
| B. Thread plus timeout | None | Cannot safely kill the native thread | Yes | REJECT: leaked thread, shared memory and side effects |
| C. Separate native plugin process with bounded IPC | Process address space, subject to OS sandbox permissions | Parent can kill/reap timed-out worker | Potentially through a child adapter | IPC validation, process-tree/OS policy and crash recovery |
| D. Wasmtime server component guest with explicit capabilities | WebAssembly guest isolation, bounded import surface | Guest fuel/epoch trap; blocking host imports need independent bounds | No, new WIT/component ABI required | Runtime dependencies, SDK, scheduling/permission integration |

**Provisional direction:** D is the preferred future format for **untrusted, explicitly server-side extension code**, conditional on a minimal reproducible spike and full source-review approval. Keep existing native shared libraries labeled **trusted local only**; never call them sandboxed. Separately decide whether to disable them for untrusted deployments or isolate the legacy C ABI using C in an explicitly opt-in helper process. This is not Paper/Bukkit/Java compatibility.

## Minimum server-side interface/security contract for a future spike

1. Prevalidate a versioned server module manifest, unique ID, command labels, permission node and imported capability set. Unknown imports/side/ABI and duplicates fail before enable.
2. Expose only typed host requests, immutable world/player snapshots and bounded log/command outputs; do not lend raw references or locks for host world, inventory, permission or JSON store state.
3. Host alone validates authenticated identity (not yet implemented), world ownership, permission context and mutating requests on the correct owner thread. Unverified guests cannot forge player/group identities.
4. Per-plugin Store with guest memory/instance/table/handle quotas, per-hook fuel plus an external responsiveness policy; bound every blocking host import and disk operation separately.
5. Treat plugin load/enable/tick/command/disable errors as a guest failure transition, unregistering owned commands. Do not let a trapped plugin contaminate another plugin's state.
6. Limit stdout/terminal control escape effects and deny WASI filesystem/network/OS access unless capability explicitly reviewed. Per-plugin data storage must have namespacing, quotas and atomic durability.
7. Preserve deterministic command ownership and capped per-tick work already provided by the server.

All of these are requirements, not achieved capabilities.

## Acceptance and falsification matrix before migration approval

| Experiment | Required observation |
|---|---|
| Guest infinite loop in init/tick/command | Bounded trap/stop of guest; main server remains responsive to status, version and stop |
| Guest memory/table/handle growth | Guest denied at configured cap; measured host RSS and cost recorded separately |
| Unknown import, wrong side/ABI, undeclared file/network access | Validation rejects before running guest callback |
| Huge/invalid command manifest, duplicate aliases | No partial registration and no privileged command injection |
| Slow/blocking host import | Independent time/I/O/queue budget; guest fuel alone cannot be credited |
| Trap while shutting down | Host completes shutdown and releases persistent locks; no leftover worker |
| Many plugins and command collisions | Deterministic ordering and resource budget accounted for |
| Existing unsafe native shared library | Must remain explicitly unsafe, even if Wasm plugins pass |
| Performance | Real release build, repeated same-host p50/p95/p99 hook time, idle/tick cost, cold start, memory and trap cost. No guessed speedup. |

Regression prerequisites: Rust suite, scripts/test_native_plugin_nontermination.py (expected exposure), scripts/test_native_plugin_order.py, scripts/test_plugin_lifecycle.py, scripts/test_startup_preflight.py, TCP/storage integration and fresh Cargo license/review gate. A spike is not a product feature and must not bypass Phase -1.

## Next steps and dependencies

Before any code change, pin the Wasmtime crate version and exact license/dependency profile, confirm component feature and toolchain, then author the smallest **server-specific** WIT interface with one log callback and one safe bounded event; do NOT reuse the Horizon client interface unchanged. Bench CPU/fuel/epoch behavior, memory and host import time. Independently decide the native .so opt-in or subprocess migration; no thread watchdog is acceptable as an isolation substitute.

The parent REV-PLUG, RV-CLOSE and nine retrospective review rows remain IN_REVIEW. This document does not authorize gameplay, server plugin API claims, a new runtime dependency, or publication as a secure sandbox.


## RV-PLUG-06 — measured native-worker boundary experiment (not integrated)

This is a disposable process feasibility harness, NOT integrated into the Minecraft Rust server. To reproduce: python3 -B scripts/experiment_native_process_boundary.py. It compiles a short first-party native C ABI test callback, launches it from an isolated Python worker after reporting a flushed entry marker, then supervises completion or kills and reaps only its own worker process group. The marker excludes Python startup time from callback observations. Core dumps are disabled.

| Callback | Trial 1 | Trial 2 | Verified outcome |
|---|---:|---:|---|
| Normal return 7 | 0.007s | 0.006s | Worker exits normally; supervisor survives |
| Direct invalid memory write | 0.059s | 0.063s | Child SIGSEGV (-11), supervisor survives |
| Infinite pause | 0.801s | 0.801s | Child SIGKILL (-9) at 0.8s callback timeout, supervisor survives |
| Write test-only host-side file | 0.006s | 0.005s | Child can access/write a file outside its plugin path with same host user privileges |

Incidental local timings are not benchmarks of production plugin execution latency or throughput. The first experimental C raise(SIGSEGV) variant stalled in this sandbox, and a text-buffered readiness read swallowed a fast normal-result marker; final passing harness uses a direct C null write and binary unbuffered marker read. Eight final trials passed. Only temporary test files and ephemeral children were used.

Option C therefore demonstrates child fault containment and supervised termination under these conditions AND demonstrates missing filesystem protection. Before claiming a secure native plugin process, it must enforce OS credentials/file/network/sandbox policy, bounded authenticated IPC, process descendant escape control, resource quotas, permission and tick ownership checks, and complete server lifecycle integration. Existing Rust .so plugins still run IN-PROCESS without containment. Option D Wasmtime remains provisional and no such engine was installed in this session. No review approval is implied.


## RV-PLUG-07 — Wasmtime 50.0.1 guest limits tested; no server-side WIT yet

A second disposable proof tested official Python wasmtime-py exactly version 50.0.1 (PyPI release dated 2026-10-22, declared license Apache-2.0 WITH LLVM-exception). The package was installed with pip --no-deps to an isolated project-local scratch and removed when the command exited; Cargo.lock and the server binary were unchanged. Reproduce by installing the pinned wheel in a throwaway directory, putting it on PYTHONPATH only for the process, and running: python3 -B scripts/experiment_wasmtime_core_limits.py.

Verified first-party WAT **core Wasm** guest behavior (not a component WIT interface):
- Explicit import host.note(7) invoked and guest returned 17.
- An infinite guest loop with 500 fuel trapped from fuel exhaustion. The local 0.0002s was a single observation, not a guarantee for arbitrary guests.
- An attempt to grow the guest's declared 1..2-page linear memory by another four pages failed with result -1.
- An unlinked wasi_snapshot_preview1.fd_write import was rejected before guest execution.
- A synchronous Python host import deliberately slept 150ms, completed successfully with 200 original guest fuel still nonzero, proving fuel does not interrupt blocking host code.
- A clean later callback succeeded following the tests.

**Decision impact:** Engine-level guest fuel, import rejection and declared memory maximum can be demonstrated today without rewriting the server or adding a Rust dependency. The blocking host-call experiment independently validates why future server host functions must be bounded for time, I/O and queue depth or kept off the tick/region owner thread. A future WIT component host still needs separate verification of actual store limiter, epoch/wall interruption, capabilities, output quotas, authentication, owner-thread message dispatch, traps and Rust crate distribution/licenses. It must NOT reuse the pinned Horizon client-side runtime as if server mods were already supported. Existing Rust .so still executes in-process without any guest sandbox.


## RV-PLUG-08 — typed component-model evidence, not server integration

A new disposable Wasmtime v50.0.1 Python experiment (scripts/experiment_wasmtime_component_contract.py) uses an actual Component Model text module, with canonical lowering of a typed host import and lifting of a typed event export. A small independent WIT-shaped server world is documented in scripts/fixtures/server-plugin-probe.wit. This WIT text does not automatically generate the executable component: the companion WAT is first-party manually written to implement the same scalar types. Passing tests do not certify full WIT bindgen compatibility or complex buffer/string ownership.

Eight experiment checks passed: declaration/shape, actual Component compile, missing host import rejection, host note(u32 7) returning guest u32 17, incorrect string parameter rejection, host function sleep 150ms completing despite fuel remaining, 500-fuel infinite guest loop trap, and independent healthy instance startup. The Python component fuel trap arrived wrapped in WasmtimeError; after recording the original unexpected exception, the fixture explicitly accepts WasmtimeError and Trap while checking the fuel cause. Package was ephemeral only; no Cargo or actual server changes.

The strongest security conclusions are limited: typed imports form an explicit host capability boundary, but an authorized synchronous host import can still occupy the server thread irrespective of guest fuel. The new WIT world does not represent a released protocol, a client/server compatible mod SDK, auth/world authority, full WASI policy or safe plugin isolation. The real Rust native .so engine remains unrestricted and nonterminating callbacks can block it. Next proof must include host-side bounded action queue and import operation budgets, plus explicit resource and lifecycle management before any server integration and REV-LEGAL review.
