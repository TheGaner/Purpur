# Purpur-Horizon master roadmap

Goal: independently runnable Rust-only dedicated server for Horizon clients with version-aligned Purpur behavior, native Rust plugins, and compatible existing Java plugins. No Horizon client/engine or Purpur/Java server executable in the runtime. The optional JVM is only for Java plugins.

Status: 2026-10-23, protocol 777 status/ping, nonplayable login rejection, simple 20Hz tick loop, Rust native plugin ABI, Java lifecycle-only bridge, now a shared Rust command registry. Not a playable game.

## 0 — Standalone foundation [PARTIAL]
- [x] Independent Rust status/ping server and tested native/JVM plugin lifecycle.
- [ ] Harden configuration, graceful shutdown, bounded connection counts, CI, reproducible packaging.
Gate: distributable release runs without Horizon engine/Java server dependencies.

## 1 — Command layer [ACTIVE]
- [x] Quoted/escaped lexer with input bounds.
- [x] Register/alias/namespaced commands, collision prevention, unregister by plugin owner.
- [x] Console help, help details, version, stop, uptime, tps (target only), echo, plugins.
- [x] Word, bounded integer, greedy arguments; contextual permission checks with default deny.
- [x] Optional native Rust ABI command declarations and demo plugin dispatch.
- [x] C-01: Deterministic, permission-filtered, versioned Brigadier-style protocol-neutral graph with canonical alias redirects, typed arguments and bounded registration; 10 new Rust tests. NOT a wire Commands packet encoder.
- [x] C-02: Bounded console command backlog (1,024) and per-tick dispatch cap (128), with overflow/tick regression tests. Player commands and suggestions still require separate queues.
- [ ] C-03: LuckPerms-style tri-state contextual permission snapshot/caching model with explicit invalidation; profile before optimizing.
- [ ] C-04: Encode protocol 777 Commands packet and implement real player command/suggestion inbound handlers (depends on Horizon login).
- [ ] Persistent groups, nodes, expiring grants, LuckPerms-style contexts and metadata/caching.
- [ ] Real player command ingress, Brigadier command tree and suggestions, client packet replies.
- [ ] Selectors, coordinates, resource IDs, NBT, complex arguments, async command tasks.
- [ ] Java plugin command registration/execution over bidirectional bridge.
- [ ] Vanilla and Purpur 26.3 command differential corpus and permission integration tests.
Gate: once play login exists, client sees permission-filtered command tree and player/console execution share one registry.

## 2 — Horizon-only play [BLOCKING]
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

## 5 — Plugin APIs, EssentialsX, LuckPerms [PLANNED]
- [ ] Shared command/event/permissions/world/player APIs and stable event cancellation/order.
- [ ] Versioned Rust-WASM plugins with Horizon-like WIT capability checks, isolation and fuel/memory budgets; native plugin dylibs remain trusted.
- [ ] LuckPerms first: native context-sensitive permission nodes, inheritance, metadata, persistence and cache invalidation. Then build the Java LuckPerms API facade and test actual JAR loading (MIT).
- [ ] EssentialsX: prefer running an unmodified Java JAR once Bukkit/Paper classloading and plugin.yml/paper-plugin.yml work. Test /home, /sethome, /warp, /spawn, /tpa, /msg, moderation, economy/Vault and persistence. Do not copy GPL source into Rust core without review.
- [ ] JVM IPC: registration, callback/response, task scheduling, command completers, permissions, classloaders, API binary compatibility matrix.
Gate: version-pinned real EssentialsX and LuckPerms integration smoke suite passes; mere Java onEnable is insufficient.

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
