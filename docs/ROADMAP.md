# Purpur-Horizon master roadmap

Goal: standalone Rust-only Horizon server with Purpur parity and ALWAYS-ON native Rust EssentialsX-style utilities and LuckPerms-style permissions; no Java JAR required. Optional external Java plugin compatibility remains a separate feature.

## -1 — Mandatory source comparison and retrospective approval [ACTIVE; BLOCKS NEW FEATURES]

**Standing rule for every change:** BEFORE implementation, inspect and compare all **materially relevant** reference implementations from Paper/Purpur, Folia, SteelMC, FerrumC, Horizon, LuckPerms, EssentialsX, and—where networking applies—Krypton, e4mc, PulseNet research. Recheck pinned source revisions, relevant classes/functions/algorithms, constraints, upstream tests, licenses, and our existing code. Document why any reference is inapplicable. Do not simply list project names.

**BEFORE writing code:** record alternative designs and a selected implementation with explicit correctness, performance, compatibility, security, source-license and rollback evidence. **AFTER writing code:** run targeted tests and comparative benchmarks, audit whether an earlier/better implementation exists, and publish the evidence and any remaining gaps **before beginning a new feature**. Best-supported is a measurable, scoped engineering conclusion; no absolute fastest/best claim without evidence.

**Retroactive freeze:** all previously implemented features are considered FUNCTIONALLY TESTED ONLY, NOT SOURCE-REVIEW APPROVED. **Do not progress into new feature milestones** (especially authentication/play, worldgen, scheduler or EssentialsX/LuckPerms feature expansion) until retrospective reviews and high-priority correctness issues are resolved or explicitly kept in a scoped review/fix milestone. Review-enabling tests, targeted fixes, and documentation are allowed; none bypass the before/after comparison.

- [x] RV-00: Locate existing roadmap, pin current HEAD, inventory all current Rust source areas, run 48/48 Rust regressions and formatting, identify existing architecture reports as preliminary evidence.
- [x] RV-01: Establish mandatory pre-/post-implementation reference-review procedure, comparison scope, licensing/benchmark policy, retrospective review ledger, and staleness checks. See [review and reference audit](IMPLEMENTATION_REFERENCE_AUDIT.md).
- [ ] RV-NET: Verify prior protocol/network choices against Horizon 26.3, SteelMC, FerrumC, Paper/Purpur, and applicable networking implementations; review deadlines, auth, memory, concurrency, p95/p99, fairness.
- [ ] RV-STATUS: Verify status/ping response framing, version behavior, limits and dynamic future semantics against Horizon and Paper/Purpur.
- [ ] RV-CMD: Differentially audit command tree, argument parsing, suggestions and permissions against Paper/Purpur Brigadier, SteelMC, FerrumC and LuckPerms.
- [ ] RV-PERM: Compare native Rust permissions to LuckPerms calculator, graph, contexts, cache and persistence, plus relevant SteelMC/Paper semantics.
- [ ] RV-UTIL: Compare homes/warps and planned player actions against EssentialsX, Bukkit/Paper, and Folia owner-affine teleport.
- [ ] RV-STORAGE: Audit JSON persistence durability and scaling against LuckPerms/EssentialsX and SteelMC/FerrumC before choosing a database design.
- [ ] RV-PLUG: Audit native ABI safety, plugin lifecycle and Java bridge against Horizon Wasmtime, Paper/Purpur/Folia and upstream plugin needs; fix unsafe metadata pointer scan and potential blocking shutdown.
- [ ] RV-TICK: Audit command work budgets, tick behavior and future ownership against Folia/Paper and SteelMC.
- [ ] RV-LEGAL: Complete per-dependency SPDX/license inventory, notices, generated assets, copied-source policy and distribution/source offer.
- [ ] RV-CLOSE: All retrospective rows APPROVED with current source digests, named source-code citations, reproducible verification and decision records. The review gate must pass; only then unfreeze new feature work.

**Enforcement:** run python3 scripts/reference_review_gate.py --check for source-file inventory correctness, then python3 scripts/reference_review_gate.py --enforce for retrospective acceptance. Enforcement is **expected to fail until the above reviews are completed**. A clean build or 48 passing tests is not a waiver. For newly changed files, re-review and record the new source digest. Upstream references and accepted design evidence go into the existing audit—not a second roadmap.

Gate: full retrospective evidence and no unresolved high-severity defects in affected modules, with measured/verified optimization decisions and preserved license provenance. This blocks proceeding just because the implementation compiles.


Status: 2026-10-23, protocol 777 status/ping, nonplayable login rejection, simple 20Hz tick loop, Rust native plugin ABI, Java lifecycle-only bridge, now a shared Rust command registry. Not a playable game.

## 0 — Standalone foundation [PARTIAL]
- [x] Independent Rust status/ping server and tested native/JVM plugin lifecycle.
- [x] NET-01: Decouple blocking accept from 20Hz tick, cap concurrent worker slots, precompute immutable status response.
- [x] NET-02: Validate protocol 777 login hello/intents and bound preauth frame sizes before allocating.
- [ ] NET-03: Deadline-aware cancellation, handshake rate and memory quotas, clean shutdown, telemetry and CI.
- [ ] Harden configuration, graceful shutdown, CI, reproducible packaging.
Gate: distributable release runs without Horizon engine/Java server dependencies.

## 0A — Licensing and source-combination prerequisites [ACTIVE]

- [x] Verify AGPL-3.0-or-later project, GPLv3 EssentialsX, MIT LuckPerms and AGPL SteelMC license families.
- [x] Document GPLv3 §13 / AGPLv3 §13 combination, per-file provenance, notices, source-access and release audit in docs/LICENSING.md.
- [x] Implement initial native Rust modules independently (no EssentialsX/LuckPerms source copied).
- [ ] Prior to ANY upstream source transplant: pin SHA, inspect copyright/exception/data/dependency licenses, choose independent vs direct-port boundary, preserve per-file terms and change logs.
- [ ] Automate Cargo license inventory/SBOM and publish matching source plus prominent AGPL network source link for deployed versions.
- [ ] Obtain qualified legal review for large GPLv3-covered transplants and distribution.
Gate: every copied file attributed, compatible, legally reviewed where needed, and covered-source release obligations fulfilled.

## 0B — Always-on native essentials and permissions [FOUNDATION IMPLEMENTED]

- [x] Native Rust permission evaluator with tri-state allow/deny/undefined, group ancestry, world context, expiring rules, revision-aware bounded cache, persistence and fail-closed invalid-data loading.
- [x] Native Rust utilities storing named home and warp locations in validated, persistent JSON; no Java, plugin loader or external service required.
- [x] Console administrative commands for permission checks/grants, group membership, home and warp storage/inspection.
- [ ] Full LuckPerms parity: exact inheritance precedence, context factories, weights, metadata, time-bound groups, async DB adapters and migrations; a test oracle.
- [ ] Full EssentialsX behavior: player-bound home/warp/spawn teleport, requests, chat, cooldowns, kits, economy, moderation, events, config and data migration.
- [ ] Transactional SQLite persistence and crash tests, robust revisions across concurrent region threads, metrics and semantic differential tests.
Gate: independent first-party Rust native services usable with real players, persistent across restarts, measured and tested against upstream behavior.

## 1 — Command layer [ACTIVE]
- [x] Quoted/escaped lexer with input bounds.
- [x] Register/alias/namespaced commands, collision prevention, unregister by plugin owner.
- [x] Console help, help details, version, stop, uptime, tps (target only), echo, plugins.
- [x] Word, bounded integer, greedy arguments; contextual permission checks with default deny.
- [x] Optional native Rust ABI command declarations and demo plugin dispatch.
- [x] C-01: Deterministic, permission-filtered, versioned Brigadier-style protocol-neutral graph with canonical alias redirects, typed arguments and bounded registration; 10 new Rust tests. NOT a wire Commands packet encoder.
- [x] C-02: Bounded console command backlog (1,024) and per-tick dispatch cap (128), with overflow/tick regression tests. Player commands and suggestions still require separate queues.
- [x] C-03a: Native tri-state context-aware permissions persistence, group inheritance, expiry-aware bounded cache and invalidation (subset only).
- [ ] C-03b: Full LuckPerms-compatible semantics plus per-context immutable snapshots and benchmarked caching.
- [ ] C-04: Encode protocol 777 Commands packet and implement real player command/suggestion inbound handlers (depends on Horizon login).
- [ ] Persistent groups, nodes, expiring grants, LuckPerms-style contexts and metadata/caching.
- [ ] Real player command ingress, Brigadier command tree and suggestions, client packet replies.
- [ ] Selectors, coordinates, resource IDs, NBT, complex arguments, async command tasks.
- [ ] Java plugin command registration/execution over bidirectional bridge.
- [ ] Vanilla and Purpur 26.3 command differential corpus and permission integration tests.
Gate: once play login exists, client sees permission-filtered command tree and player/console execution share one registry.

## 2 — Horizon-only play [BLOCKING]
- [x] Pre-auth packet validation and explicit rejection at login stage (NOT an authenticated client join).
- [ ] Minecraft 26.3 protocol 777 login/auth/encryption, configuration, secure session, play.
- [ ] Client registry negotiation, chunk streaming, movement, inventory, teleport, keepalive.
- [ ] Versioned Horizon client-server capability challenge; do not trust minecraft:brand alone.
Gate: real Horizon client connects; non-Horizon rejected with negotiated handshake, not brand spoof.

## 3 — SteelMC world generation [PLANNED]
- [ ] Pin SteelMC master to an immutable SHA and record every copied file's license/provenance.
- [ ] Reuse selected steel-worldgen noise, density, biomes, surface, structures and generated registry assets with AGPL notices. Needs dependencies, generated assets, pinned nightly SIMD audit.
- [ ] Define WorldgenProvider(seed, dimension, chunk_pos) producing an immutable chunk; delayed deterministic commit on region-owner thread.
- [ ] Bench-driven optimizations: batched/SIMD density, per-worker arenas, immutable biome/noise caches, deduplicated in-flight chunks, bounded worker pool and asynchronous stage DAG.
- [ ] SteelMC reports 7,500 vanilla-checked chunks (2,500/dimension); treat that as baseline evidence only for its unmodified pinned revision. Do not rerun full 7,500 for every change. Run small seeded goldens and changed-path boundary regression tests, larger periodic randomized parity, and a full suite at version shifts.
- [ ] Measure p50/p95 chunkgen latency, allocation count, TPS effect and output identity. New optimization requires evidence of speed-up and targeted parity.
Gate: playable correct chunks across all dimensions and reproducible benchmark improvements. No worldgen code imported yet.

## 4 — Vanilla and Purpur simulation [PLANNED]
- [ ] Persistent NBT/chunks/region data, blocks, shapes, fluid/light, entities, collision, combat, redstone, inventory, recipes.
- [ ] Commands/selectors/scoreboard/advancements, game rules and Purpur configs.
- [ ] Seed and packet replay oracle for original Purpur 26.3 behaviors; differential acceptance, tests per toggle.
Gate: survival, commands and configuration parity for documented subset.

## 5 — Extensibility and optional Java API compatibility [PLANNED]
- [ ] Shared command/event/permissions/world/player APIs and stable event cancellation/order.
- [ ] Versioned Rust-WASM plugins with Horizon-like WIT capability checks, isolation and fuel/memory budgets; native plugin dylibs remain trusted.
- [ ] Expose the built-in Rust permission service to native Rust/WASM and optional Java-compatible plugins; original LuckPerms JAR not required.
- [ ] Expose native home/warp/utilities as first-party APIs; do not require an EssentialsX JAR. Optional original EssentialsX plugin compatibility must handle duplicate services and commands explicitly.
- [ ] JVM IPC: registration, callback/response, task scheduling, command completers, permissions, classloaders, API binary compatibility matrix.
Gate: native built-ins remain active without plugins; optional Java plugins can access an API facade without replacing core implementations.

## 6 — Folia-style region scheduler [PLANNED]
- [ ] Region ownership over nearby loaded chunks, owned entity state, 20TPS worker scheduler.
- [ ] Cross-region message passing, migration, region/entity/global schedulers, safe handoff and constraints.
- [ ] Keep a single-thread compatibility mode for legacy Java Bukkit/Paper plugins. Region parallelism requires explicit plugin opt-in and strict ownership; Folia itself warns ordinary Paper plugins are incompatible by default.
Gate: deterministic cross-region packet/entity/command tests and measurable gains for spread-out players.

## 7 — Hardening, performance and release [PLANNED]
- [ ] Source-attributed license inventory and reproducible source offer (AGPL §13).
- [ ] Fuzz protocol and commands; replay differential tests; security review of JNI/JVM bridge and native plugin trust.
- [ ] Benchmark tick p50/p95, multi-player throughput, worldgen and plugin call latency.
- [ ] CI and release matrix with honest feature parity report.

References: PurpurMC/Purpur; ludovic111/horizon; EssentialsX/Essentials; LuckPerms/LuckPerms; Steel-Foundation/SteelMC; ferrumc-rs/ferrumc; PaperMC/Folia.

## 2026-10-23 architecture research checkpoint

Source implementation comparison and recommendation: docs/REFERENCE_ARCHITECTURE_RESEARCH_2026-10-23.md. Research confirmed Paper/Steel command-tree semantics, Steel generation-stage and queue designs, Folia region ownership, LuckPerms cache invalidation, Horizon WASM host budgets, and EssentialsX command reliance on Bukkit/Paper events.

Initial baseline 16/16 tests; after C-01/C-02 the repository has 26 tests passing at the first implementation checkpoint. No real-world performance improvement or Java/Purpur behavioral parity has been established. Next unblocked work is C-03 or network status/login scaffolding; prioritize C-03 only if it does not delay a first real Horizon player connection.


## 2026-10-23 native built-in pivot

This supersedes the Java-first EssentialsX proposal in the earlier research report. No original EssentialsX/LuckPerms/SteelMC implementation has been copied. Baseline was 26 tests; native permission, persistence and utility tests raise it to 36 passing Rust tests. This is an initial data and console administrative milestone, NOT playable EssentialsX or full LuckPerms parity. Next: live no-JVM smoke and Horizon login/configuration, then wire native utility actions into actual player-owned world state.

### Native core integration verification (2026-10-23)

Full local Rust suite: 36/36 passing. Release build succeeded with warnings for future player APIs. Smoke-tested compiled executable from a separate temporary directory, empty plugin folder and Java bridge off: grants and home/warp data persisted over two processes. Deliberately malformed permissions.json correctly blocked restart without clearing user data. Real logged-in player commands, teleports, database scale benchmarks, GPL-derived source combinations and deployment source-offer delivery remain unverified/pending.

## 2026-10-23 network optimization checkpoint

Full investigation, measured results and source constraints: docs/NETWORK_OPTIMIZATION_2026-10-23.md. On a localhost status-only workload, sequential p50 fell from 49.996ms to 0.117–0.128ms, and 8-way concurrent p95 from 50.506ms to 1.522–1.750ms after moving socket acceptance off the 20Hz simulation tick. These numbers are not gameplay/TPS claims; background builds and limited samples prevent production conclusions. 48 Rust tests pass, with no external source copied. Horizon auth/configuration and real play remain blocking milestones.
