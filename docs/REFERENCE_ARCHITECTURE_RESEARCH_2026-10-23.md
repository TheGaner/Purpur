# Reference server architecture comparison and design decision

Date: 2026-10-23
Project: Purpur-Horizon, standalone Rust server, Horizon-only admission and Purpur gameplay/plugin compatibility.
Purpose: Examine how each requested implementation solves commands, world generation, plugin APIs, permissions and concurrency, then select a measurable design appropriate to this project's actual pre-playable state.

## 1. Sources and confidence

Source-inspected findings below cite concrete checked-in paths. An upstream README claim is not a reproduced benchmark. Recommendations and future architecture sketches are hypotheses until built and tested. No source from these projects has been copied into the new implementation.

- Purpur 26.3: https://github.com/PurpurMC/Purpur/tree/5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0 ; see purpur-server/src/main/java/org/purpurmc/purpur/PurpurConfig.java and PurpurWorldConfig.java.
- Paper official Brigadier docs: https://docs.papermc.io/paper/dev/command-api/basics/introduction/ and https://docs.papermc.io/paper/dev/command-api/basics/command-tree/.
- Folia: https://github.com/PaperMC/Folia/tree/8cffa9059ad6bbc2b28510c38ed61a51098d4971 ; https://docs.papermc.io/folia/reference/overview/ ; https://docs.papermc.io/folia/reference/region-logic/.
- SteelMC: https://github.com/Steel-Foundation/SteelMC/tree/dc8eaba7ff21c348226a05741c2fb9809a8ca90d ; steel-core/src/command/brigadier/dispatcher.rs, steel-core/src/command/protocol.rs, steel-core/src/command/queue/requests.rs, steel-core/src/permission/manager.rs, steel-core/src/worldgen/generator/vanilla.rs, steel-worldgen/src/noise/noise_chunk.rs.
- FerrumC: https://github.com/ferrumc-rs/ferrumc/tree/51830e99675deae1f6cdbb62bf5d32b3d25c1c76 ; src/lib/commands/src/graph/mod.rs, src/lib/world_gen/src/lib.rs and src/lib/plugins/src/lib.rs. Master and rework/v2 may differ.
- Horizon: https://github.com/ludovic111/horizon/tree/e325fa65878071d5c9076e908e9a4adb258b3313 ; crates/horizon-mods/src/runtime.rs and manifest.rs; crates/horizon-client/src/net.rs; crates/horizon-protocol/.
- LuckPerms: https://github.com/LuckPerms/LuckPerms/tree/b93a4d7b24fbcb97170e355d651984493ab64e79 ; common/src/main/java/me/lucko/luckperms/common/calculator/PermissionCalculatorBase.java, cacheddata/AbstractCachedDataManager.java and graph/Graph.java.
- EssentialsX: https://github.com/EssentialsX/Essentials/tree/e70bdb817c758474acb3a23a37cf961b2f716a50 ; Essentials/src/main/java/com/earth2me/essentials/commands/EssentialsCommand.java and Commandhome.java.

## 2. What each project actually does

### Purpur, layered on Paper

Verified from Purpur source: Purpur adds a global YAML-backed PurpurConfig, per-world PurpurWorldConfig, registers some commands through the inherited Bukkit CommandMap and applies behavioral changes through patches on Paper/Minecraft. World-specific config reads defaults and named-world overrides. This is Java code tied to Bukkit APIs and Minecraft server implementation, not a portable worldgen or Rust event engine.

Verified from Paper docs: modern Paper exposes the Minecraft Brigadier command tree, including argument node types, server-side requirements and client-side suggestions. Supporting Java plugin commands also means supporting old Bukkit command interfaces and their lifecycle.

For us: treat Purpur as an executable differential behavior oracle and configuration schema target, not code to run in the server process. Preserve equivalent world-default/override semantics and make all plugin-visible command/event changes pass through one Rust-owned API.

### Folia

Verified from Folia technical docs: chunk positions have exactly one region owner; nearby chunks merge into independent regions, which tick in parallel with individual deadlines. Ownership checks prohibit one ticking region from mutating another. A global region owns time/weather/gamerules and console commands. Region and entity schedulers route work to the current owner; migrating entities need stable handles and deferred messages. Regions merge and split as chunk ownership changes.

Performance implication (inference): concurrency wins when players are sufficiently spatially separated; overhead can hurt small, tightly clustered servers. Folia's own compatibility docs warn ordinary Paper plugins are not safe under independently concurrent ticks.

For us: start with one ownership lane and an explicit region task/messaging API; evolve into partitioned world ownership only after correct world simulation. Offer compatibility mode for Java plugins and opt-in parallelism for safe Rust/WASM plugins. Never let Java plugin callbacks synchronously mutate foreign region state.

### SteelMC

Verified from source: its Rust Brigadier dispatcher owns an indexed node arena, typed arguments, parsed contexts, suggestion building, and a separate projection to the Minecraft command protocol. Its queue has separate execution and suggestion capacities; repeated suggestions can be coalesced per sender, and a bounded number of requests are processed before a tick. Permission groups are resolved through a manager that supports typed snapshots and persistence-first updates.

World generation: SteelMC's NoiseChunk evaluates interpolation channels at cell corners, using contiguous channel data, precomputed vertical coordinates, and SIMD-oriented paths while preserving trilinear interpolation semantics. Generation has separate biome, structures, terrain, features and lighting stages. SteelMC reports 7,500 matching reference chunks and a focused benchmark in its own README, which is upstream evidence for the pinned version rather than our own reproduction.

For us: this is the best current Rust reference for target-version vanilla parity, provided AGPL obligations are honored. Reuse selected generator implementation and dependencies, not the full server. Keep seed/registry-sensitive calculations exact, and optimize only after a reproducible same-version baseline.

### FerrumC

Verified from source: its command graph is a vector of nodes with indexed children, maps command parts to indices, and sends a graph to players. Its master worldgen is a climate-driven, multi-stage terrain system that fills/carves surfaces, picks biomes, floods, caves and decorates with per-chunk local arrays. Its plugin crate in the inspected master is currently a placeholder. A source README describes a significant rewrite on a separate branch; do not mistake the rework claims for what the inspected master implements.

For us: useful comparison for Rust data layout, world/chunk storage and ECS ideas, but the climate/terrain algorithm is not demonstrated as a 26.3 Purpur-equivalent replacement. Do not combine outputs from two different generators within a seed without explicit versioning and correctness evidence.

### Horizon

Verified from source: Wasmtime component-model mods have a versioned manifest, declared imports/capabilities, fuel, storage, memory and per-call resource limits, with trapping modules disabled. Server-side mod execution is described but not yet active in the inspected manifest. Horizon's current client uses Minecraft protocol semantics; the brand payload is not proof of client identity.

For us: adapt the WIT component host to server side with an explicit API for registering commands, reading immutable snapshots, and enqueueing authorized mutations. A native Linux Rust dylib is an opt-in trusted extension, not sandboxed. The Horizon-only guarantee requires new explicit client-server negotiation and actual cryptographic/authenticated session binding, not just checking a user-provided brand field.

### LuckPerms

Verified from source: PermissionCalculatorBase evaluates an ordered processor chain and uses a loading cache of permission results, invalidated when the underlying effective permissions change. CachedData managers cache per QueryOptions (context). Group relationships use graph traversal. This is more complex than testing the longest matching wildcard string.

For us: define a tri-state permission result (allow/deny/undefined), immutable effective permission snapshots for each subject and context, revision-based invalidation, and explicit group cycles/tie semantics. Retain a deny-by-default policy for commands that require permission. Ensure authorization is rechecked when executing, not only when presenting client command trees.

### EssentialsX

Verified from source: command classes extend EssentialsCommand and use Bukkit user/server interfaces. Commandhome alone relies on permission checks, persistent user home data, PaperLib asynchronous calls, cancellable teleport events and async teleport completion futures. Pure command-name registration is nowhere near Java plugin compatibility.

For us: define player, world, teleport, persistence, command, events, economy and scheduling APIs in Rust. Bridge real Bukkit interfaces via an isolated JVM process and asynchronous request/response protocol; only then load an unmodified EssentialsX jar and verify end-to-end behavior.

## 3. Recommended hybrid architecture

    Horizon 26.3 client
        |
        +-- authenticated connection and Horizon capability negotiation
        |
    Bounded Rust network queues / protocol serializer
        |
    Command tree + context-aware authorization + event dispatcher
        |
    Immutable server action requests (command, packet, scheduled mutation)
        |
    World-ownership actor(s) -- 20 TPS budget and deterministic ownership
       |                  |
       |                  +-- async worldgen workers (immutable chunk output)
       |                  +-- async disk/NBT IO, bounded and ordered commits
       |
       +-- Native Rust plugins (trusted opt-in) / WASM capability host
       |
       +-- JVM sidecar: Bukkit/Paper/Purpur facade / EssentialsX / LuckPerms
                via versioned asynchronous RPC, not direct Rust world pointers

Ownership rule: each mutable chunk, entity and inventory belongs to one actor. Cross-region commands and teleports create messages containing an owner/version, never borrowed mutable pointers. Strict single-owner order is the correctness baseline. Region parallelism becomes a selectable scheduling optimization once world state exists.

## 4. Best plausible optimized approach, without invented benchmarks

Command dispatch:
- Registry mutations build a versioned immutable arena; client command-tree serialization is permission-filtered, deterministic and cacheable only under an explicit registry + permission/context revision key.
- Typed parsers should operate on borrowed input slices rather than allocate owned strings at every node.
- Convert hot-path commands into compact internal action IDs after permission checks; do not involve JVM RPC for native server actions.
- Separate bounded execution and suggestion queues. Coalesce duplicate stale suggestion requests per sender; retain execution budget independent of suggestions.
- Tristate permission cache should be scoped by subject, world/context, policy revision and expiry. Do not cache permissions across changes without invalidation. Memoization and graph closure should be profiled, not just assumed beneficial.
- Serialize command trees once per effective permission snapshot and registry revision for reconnects, with a bounded cache (no global unbounded user-specific trees).

Worldgen:
- Reuse exact pinned SteelMC generation algorithms and stage order first. Seed and RNG stream identity have precedence over SIMD throughput.
- Use structure/biome/noise work stages with bounded worker pools; deduplicate in-flight chunks and carry immutable results into world ownership lanes.
- Candidate optimizations: reuse per-worker scratch arrays; precompute constant noise graph nodes; batch SIMD corner evaluations; cache only explicitly pure climate/noise nodes with dimensions/seed/version in the key; avoid lock contention by sharding results.
- Recheck changed domains with targeted goldens, neighbor seams, multiple seeds and borders. Full 7,500 upstream chunk verification is for initial port, upstream version changes or major rewrites, not for every localized optimization. The prior upstream result alone does not validate a new port.

Scheduler:
- Start with actor/ownership abstraction before full Folia splitting. When regions exist, tick ready independent regions in parallel and keep all cross-region work in queues.
- Prefer work-stealing only for independent async generation/IO; region mutability cannot be made safe simply by using a work-stealing executor.
- Maintain a strict legacy Java compatibility lane and require explicit region-safe plugin capability for concurrent region hooks.
- Avoid exposing arbitrary synchronous world calls across an IPC boundary that could deadlock world owners.

Plugin performance:
- Precompile and pool WASM components where safe. Limit fuel, memory, I/O and per-plugin time; isolate capabilities; never implicitly trust dylibs.
- Batch read-only snapshot data and asynchronous mutation requests for Java plugins. Measure round-trip callback latency before setting a synchronous event bridge policy.

## 5. Alternatives rejected or deferred

- A Rust rewrite that launches Horizon's integrated engine: fails standalone objective.
- Hosting Purpur Java as a hidden game simulation process: violates Rust-only simulation.
- Directly transplanting all Folia internals before world state exists: increases concurrency bugs and breaks legacy plugins without player-visible benefit.
- FerrumC's current climate-driven terrain replacing SteelMC parity worldgen: unverified vanilla/26.3 parity.
- Eagerly porting EssentialsX implementation into AGPL Rust: unnecessarily creates GPL/API maintenance complexity; first support the existing Java plugin.
- Assuming the current flat tokenizer is a complete Brigadier/26.3 command protocol parser: it is not.

## 6. Execution order and gates

C-01 [DONE THIS SESSION] Versioned, deterministic, permission-filtered protocol-neutral tree with typed arguments, canonical aliases, bounded registrations and tests. Implemented in src/command/tree.rs and src/command.rs. No network command packet encoding is claimed.

C-02 [DONE THIS SESSION] Bound console backlog to 1,024 messages and drain at most 128 per tick; tests cover overflow and tick budget. Player execution and suggestion queues are not yet implemented.

C-03 [NEXT, INDEPENDENT] Tri-state/context-aware permission snapshot model with explicit cache invalidation and deterministic group cycles. Tests: user/group/context precedence, invalidation, expiry and any deny short circuit. Benchmark fresh/cached checks before accepting an optimization.

P-01 [BLOCKING FOR PLAY] Horizon 26.3 login/configuration protocol vertical slice with a real client and a clearly versioned capability exchange. Reject non-Horizon by a negotiated feature contract backed by the authenticated session; note the protocol alone cannot prove an unmodified official client.

C-04 [AFTER P-01] Client Commands packet serializer based on C-01 plus serverbound executable command and suggestion packet handlers. Test output against Horizon protocol IDs and Java26.3 fixture messages and ensure command validation still happens on execution.

W-01 [AFTER P-01 MINIMAL CHUNKS] Pin/port SteelMC worldgen under AGPL with required registries/dependencies. Verify version-aligned goldens before optimizing.

J-01 [AFTER C-04 AND PLAYER API] JVM sidecar messaging and a narrow Bukkit command/player facade, then LuckPerms API; next integrate EssentialsX service surface (home/warp/teleport) with real JAR tests.

R-01 [AFTER WORLD OWNERSHIP] Single-owner world scheduler, then opt-in Folia-like split/merge and region/entity/global queues; verify handoff correctness before speed claims.

## 7. Verification and benchmark contract

Completed today: baseline had 16 Rust tests; command tree, live console projection diagnostic and bounded queue increments bring that total to 26. Tests are unit/socket/smoke of the current prototype, not actual Horizon client join or worldgen.

Before asserting that the hybrid is fastest or more scalable, collect comparable measurements:
- Parse/permission/execute commands-per-second, allocation count and p50/p95/p99; include denied and cold-cache paths.
- 20TPS tick deadline miss rate and time spent in command callback, plugin callback, and action commit.
- Worldgen fresh-chunk throughput, median/p95 latency, memory per worker and exact seed/hash parity.
- Small crowded world vs many spread-out players to determine region scheduling break-even point.
- JVM async callback queue latency and WASM fuel/time metrics under malformed plugin inputs.

Run pin-fixed workloads on unchanged hardware and commit benchmark manifests/logs. Treat source-upstream performance claims separately from our own measured results.

## 8. Licensing and unresolved risks

The current code is AGPL-3.0-or-later; SteelMC is AGPL-3.0-or-later; Horizon, Purpur, LuckPerms and FerrumC use MIT; Folia patches and EssentialsX have GPLv3 obligations. Every directly imported file needs provenance and notice preservation; optional Java JAR execution does not erase its licensing.

Risks: no real login/play loop; no Bukkit compatibility; worldgen dependencies include generated assets and nightly SIMD; older client packet IDs may differ from latest 26.3; generated tree is protocol-neutral, not Commands serialization; group permission precedence is still a prototype and not LuckPerms-parity. Address these before promising drop-in plugin compatibility.


## 9. Observed verification checkpoint

- Host-side cargo test --all-targets (Rust 1.90 local toolchain): 26 passed, 0 failed.
- cargo fmt --all -- --check: passed.
- cargo build --release: passed; compiler emitted expected dead-code warnings for future in-game player and permission APIs.
- Live console commandtree returned a valid 19-node graph and 15 root labels with the example Rust plugin loaded.
- Live native plugin enable/command/disable and Java test JAR enable/disable smoke tests passed.
- No Minecraft client join, in-game command tree packet, full Bukkit plugin, world generation, native-world simulation or comparative performance measurement has passed because those systems do not yet exist.
