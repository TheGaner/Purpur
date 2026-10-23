# Purpur-Horizon master roadmap

Goal: standalone Rust-only Horizon server with Purpur parity and ALWAYS-ON native Rust EssentialsX-style utilities and LuckPerms-style permissions; no Java JAR required. Optional external Java plugin compatibility remains a separate feature.

## -1 — Mandatory source comparison and retrospective approval [ACTIVE; BLOCKS NEW FEATURES]

**Standing rule for every change:** BEFORE implementation, inspect and compare all **materially relevant** reference implementations from Paper/Purpur, Folia, SteelMC, FerrumC, Horizon, LuckPerms, EssentialsX, and—where networking applies—Krypton, e4mc, PulseNet research. Recheck pinned source revisions, relevant classes/functions/algorithms, constraints, upstream tests, licenses, and our existing code. Document why any reference is inapplicable. Do not simply list project names.

**BEFORE writing code:** record alternative designs and a selected implementation with explicit correctness, performance, compatibility, security, source-license and rollback evidence. **AFTER writing code:** run targeted tests and comparative benchmarks, audit whether an earlier/better implementation exists, and publish the evidence and any remaining gaps **before beginning a new feature**. Best-supported is a measurable, scoped engineering conclusion; no absolute fastest/best claim without evidence.

**Retroactive freeze:** all previously implemented features are considered FUNCTIONALLY TESTED ONLY, NOT SOURCE-REVIEW APPROVED. **Do not progress into new feature milestones** (especially authentication/play, worldgen, scheduler or EssentialsX/LuckPerms feature expansion) until retrospective reviews and high-priority correctness issues are resolved or explicitly kept in a scoped review/fix milestone. Review-enabling tests, targeted fixes, and documentation are allowed; none bypass the before/after comparison.

- [x] RV-00: Locate existing roadmap, pin current HEAD, inventory all current Rust source areas, run 48/48 Rust regressions and formatting, identify existing architecture reports as preliminary evidence.
- [x] RV-01: Establish mandatory pre-/post-implementation reference-review procedure, comparison scope, licensing/benchmark policy, retrospective review ledger, and staleness checks. See [review and reference audit](IMPLEMENTATION_REFERENCE_AUDIT.md).
- [ ] RV-NET: Verify prior protocol/network choices against Horizon 26.3, SteelMC, FerrumC, Paper/Purpur, and applicable networking implementations; review deadlines, auth, memory, concurrency, p95/p99, fairness.
  - [x] RV-NET-01: Cross-reference FerrumC whole-handshake timeout and Horizon connection semantics; replace per-read timeout loophole with absolute preauth deadline and slow-drip regression; wider REV-NET still IN_REVIEW.
  - [x] RV-NET-03: Compare FerrumC whole-handshake timeout and Horizon stream writers; extend absolute three-second admission deadline to all outbound status/login rejection writes, with existing one-second optional Pong grace. Three new real-TCP writer tests including blocked-receiver pressure; 77 Rust tests. Wider REV-NET IN_REVIEW.
  - [x] RV-NET-04: Compare FerrumC preauth timeout, Horizon transport and Paper per-IP throttle (not the same as concurrency quota); prevent one IP exhausting all global handshake slots with configurable --max-per-ip=8 default, canonical IP quota + RAII release. 3 new Rust regressions (80 total), 16-thread contention, real release-process shared/different loopback IP tests. Wider REV-NET IN_REVIEW.
  - [x] RV-NET-05: Retrospective measured admission fairness under all eight held per-IP slots (global 16): two local trials, 16/16 quota refusals, 48/48 successful distinct-IP responses under contention, 24/24 recoveries, and unloaded comparison/quantiles. Added reproducible scripts/bench_admission.py; no Rust runtime change and no DDoS/TPS claims. Wider REV-NET remains IN_REVIEW.
- [ ] RV-STATUS: Verify status/ping response framing, version behavior, limits and dynamic future semantics against Horizon and Paper/Purpur.
  - [x] RV-STATUS-01 / RV-NET-02: Compare Horizon's separate status and Ping packets and FerrumC timeout strategy; bound optional Ping wait to one second after status. Reproduced idle-worker hold 3.035s before and ~1.02s after; delayed Pong and all Rust regressions pass. Wider reviews IN_REVIEW.
- [ ] RV-CMD: Differentially audit command tree, argument parsing, suggestions and permissions against Paper/Purpur Brigadier, SteelMC, FerrumC and LuckPerms.
  - [x] RV-CMD-01: Compare pinned SteelMC Brigadier normalization and visible command tree plus LuckPerms permission processors; reproduce and repair inconsistent mixed-case command identity, partial-allow from truncated legacy group walk and accepted inverted integer range. 4 new Rust regressions (69 total), release and integration checks. Broader REV-CMD remains IN_REVIEW.
- [ ] RV-PERM: Compare native Rust permissions to LuckPerms calculator, graph, contexts, cache and persistence, plus relevant SteelMC/Paper semantics.
  - [x] RV-PERM-01: Compare pinned LuckPerms calculator, cache, storage and SteelMC group snapshots; fix mixed-case legacy subject/group access and migrate unambiguous v1 names in memory while refusing colliding identities. Three tests, 60 total passed. Broader REV-PERM remains IN_REVIEW.
  - [x] RV-PERM-02: Reproduce partial Allow on group ancestry saturation and cross-user inherited grants through malformed JSON; return Undefined on over-limit traversal and reject invalid subject/parent namespaces on open. 62 Rust tests passed; full REV-PERM still IN_REVIEW.
  - [x] RV-PERM-03: Review and fix console typed identity aliases (user:alex / alex and group:staff / staff), without crossing namespaces; reject nested or invalid groups and verify real process clean/crash restarts. Full LuckPerms semantics still IN_REVIEW.
  - [x] RV-PERM-04: Compare pinned LuckPerms caches/calculators and SteelMC validated permission configuration; reproduce and refuse arbitrarily long or malformed permission-node/world lookup strings before cloning/caching them (8192-entry cache had unbounded key bytes). Two new Rust tests including 128-byte valid boundaries (82 total), no protocol/gameplay feature. REV-PERM still IN_REVIEW.
- [ ] RV-UTIL: Compare homes/warps and planned player actions against EssentialsX, Bukkit/Paper, and Folia owner-affine teleport.
  - [x] RV-UTIL-01: Compare pinned EssentialsX UUID user files and home-name normalization with native JSON v1; reproduce inaccessible mixed-case imported home/warp records and ambiguous aliases, safely normalize valid legacy keys on open and reject colliding user/home/warp keys. Two new tests (71 total), real server import/rewrite and fail-closed startup validated; broader REV-UTIL IN_REVIEW.
- [ ] RV-STORAGE: Audit JSON persistence durability and scaling against LuckPerms/EssentialsX and SteelMC/FerrumC before choosing a database design.
  - [x] RV-STORAGE-01: Source compare pinned LuckPerms per-path locks and backends, EssentialsX UUID player files, FerrumC LMDB and SteelMC region storage; implement single-writer JSON safety, durable-rename improvements, output-size cap and real two-process persistence tests. The broader storage review remains IN_REVIEW.
  - [x] RV-STORAGE-02: Recover safely from ambiguous fsync-after-rename by phase-tagging write errors, poisoning in-process permission/utility stores, denying/hiding stale reads and blocking writes until reopen; verified with injected faults and SIGKILL restart. Wider REV-STORAGE stays IN_REVIEW.
- [ ] RV-PLUG: Audit native ABI safety, plugin lifecycle and Java bridge against Horizon Wasmtime, Paper/Purpur/Folia and upstream plugin needs; fix unsafe metadata pointer scan and potential blocking shutdown.
  - [x] RV-PLUG-01: Compare Horizon Wasmtime, Paper Java lifecycle, FerrumC stub and SteelMC command API; remove unbounded native manifest pointer scan with host-owned v2 buffer and cap Java child shutdown, unit/runtime fixtures; wider REV-PLUG still IN_REVIEW.
- [ ] RV-TICK: Audit command work budgets, tick behavior and future ownership against Folia/Paper and SteelMC.
  - [x] RV-TICK-01: Compare pinned SteelMC command queues, Paper tick schedules and official Folia independent region deadlines; reproduce slow-command tick monopolization and add a cooperative 10ms/128-command console budget with pre-dequeue cutoff, Stop/FIFO and zero-budget progress tests (74 Rust tests). Full REV-TICK IN_REVIEW.
- [ ] RV-LEGAL: Complete per-dependency SPDX/license inventory, notices, generated assets, copied-source policy and distribution/source offer.
  - [x] RV-LEGAL-01: Verify pinned upstream license files where available; enumerate all 11 current Cargo.lock registry crates and SPDX metadata, add deterministic offline Cargo license/checksum drift gate, three unit tests and THIRD_PARTY_NOTICES table. Full release license-text, source offer and asset obligations still open; REV-LEGAL IN_REVIEW.
  - [x] RV-LEGAL-02: Reproduce that the ordinary retrospective --check accepted stale or missing current_sha256 for in-review Rust/Python source. Enforce current digests on in-review/approved entries while keeping pending reviews eligible and --enforce blocked; three isolated Python tests and updated review ownership.
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

## 2026-10-23 retrospective remediation checkpoint

REV-PLUG and REV-NET are IN_REVIEW, NOT APPROVED. Targeted unsafe native manifest pointer-read and hanging Java process fixes, plus deadline-aware preauthentication reads, passed 51/51 Rust tests, release builds, normal/hung Java fixture lifecycle and two repeated status-only benchmarks. New v2 native manifest ABI requires recompiling existing native plugin examples; v1 event hooks remain. Reference comparison, source hash and outstanding risk evidence are in docs/IMPLEMENTATION_REFERENCE_AUDIT.md. The Phase -1 hold against unrelated feature development remains in force.

## 2026-10-23 retrospective single-writer persistence checkpoint

RV-STORAGE-01 completed as **focused hardening**, not full review approval. Native JSON services now lock their data files against simultaneous active writers, write uniquely named temporary files without truncation, reject writes exceeding the 2 MiB restart limit and sync the renamed directory on Unix. The JSON v1 schema remains compatible. Full Rust test suite: 54 passing (51 at session start); release build, explicit corruption/restart tests, two-process exclusion/restart test, Java lifecycle smoke and source-review coverage check passed. Ambiguous post-rename sync failures, filesystem crash simulation, migration, large-store scaling and full LuckPerms/EssentialsX semantics remain open in docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Phase -1 hold remains enforced.


## 2026-10-23 retrospective ambiguous-write recovery checkpoint

RV-STORAGE-02 was completed as a narrow safety fix, NOT complete storage approval. Rust 1.90 full test suite 57/57 passed (54 starting baseline); release build, rustfmt, real two-instance write rejection and clean/unclean process restarts, normal/hung Java sidecar regression passed. An injected directory-sync failure after successful rename now leaves permissions fail closed and utilities unavailable until reopen, preventing a stale live snapshot from overwriting disk. Independent pre-rename failures remain retryable. The repository retains native JSON v1 compatibility and first-party AGPL source; no real power-loss test, transaction migration or performance benchmark occurred. See docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Phase -1 mandatory review hold remains ACTIVE.


## 2026-10-23 native permissions case-identity checkpoint

RV-PERM-01 closed as a focused correctness fix after RV-STORAGE-02. Case-insensitive name-based subjects/group links now resolve identically; old non-colliding mixed-case JSON v1 files remain readable, and potentially conflicting historic grants fail closed rather than merging. Three added tests brought the Rust suite to 60 passed, and real-process mixed-case lookup and persistence smoke passed; this is not real player/UUID authentication and does not establish full LuckPerms or Bukkit parity. Detailed source comparison and limitations in docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Mandatory nine-area retrospective gate is still active.


## 2026-10-23 bounded permission graph security checkpoint

RV-PERM-02: confirmed and fixed two inherited-permission safety bugs using failing-before/passing-after tests. A graph larger than 128 reachable subjects can no longer produce an Allow based on partial traversal, and manually persisted parent links that cross from group inheritance into another user identity are rejected. The JSON v1 file format is preserved. 62 Rust tests pass; these results cover the prototype's fail-closed behavior, not full LuckPerms or SteelMC semantic equivalence. Full review gate remains ACTIVE; pinned source comparisons and unverified cases are in docs/IMPLEMENTATION_REFERENCE_AUDIT.md.


## 2026-10-23 scoped permission namespace checkpoint

RV-PERM-03 fixed previously confirmed typed principal inconsistencies. The old evaluator looked up user:user:alex after a console grant for user:alex, and the parent updater constructed group:group:staff when passed group:staff. A single validated one-prefix parser now preserves consistent user-only lookups, typed group parent aliases and fail-closed invalid input. Unit tests and real-console restart verification cover the change, not authenticated identities or full LuckPerms precedence. The mandatory Phase -1 cross-server source review remains active, and the code stays pre-playable. Details and source references: docs/IMPLEMENTATION_REFERENCE_AUDIT.md.


## 2026-10-23 status-only worker occupancy checkpoint

RV-NET-02 / RV-STATUS-01: status-only TCP clients previously held a network worker for the entire 3-second preauth admission deadline after status was already sent. A before-fix regression measured 3.035s; a status-specific one-second Ping grace now releases a status-only worker after about 1.02s on localhost and preserves 180ms delayed Ping/Pong. This is not proof of higher production throughput, internet compatibility at extreme RTT or gameplay TPS. Horizon protocol and FerrumC connection references, rationale and remaining tradeoffs are in docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Phase -1 source review is still active.


## 2026-10-23 scoped command registry correctness checkpoint

RV-CMD-01 advanced the Phase -1 command audit without unfreezing new gameplay. Three failing-before issues were corrected: registered commands now consistently use canonical lowercase identity for their routes and effects; the legacy in-memory Grants helper never returns Allow from an unvisited suffix of the group graph (bounded 64 subjects, including default); impossible registered integer min/max descriptors reject before mutation. A nested fanout/cycle boundary test was added. 69 Rust unit tests passed and release compilation succeeded. The implementation is still NOT an actual protocol-777 Commands packet or complete Brigadier/LuckPerms semantics; see docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Review hold ACTIVE.


## 2026-10-23 imported utilities v1 JSON compatibility checkpoint

RV-UTIL-01 completed targeted retrospective correctness hardening. Previously valid uppercase legacy home/warp/user map keys were invisible to lowercase lookups and could coexist with lowercase aliases after a later save. NativeUtilities now casefolds unambiguous v1 keys on open, rejects collisions in all three namespaces, and preserves original world identifiers/coordinate values. New tests raised the Rust suite to 71 passing; an actual server process imported, read and rewrote a legacy file and rejected an intentionally ambiguous warp on restart. This remains a names-based persistent console data subset with no actual EssentialsX teleport, UUID authentication or Folia owner-affine scheduling. The mandatory Phase -1 review hold remains active; source comparison and evidence in docs/IMPLEMENTATION_REFERENCE_AUDIT.md.


## 2026-10-23 tick console fairness checkpoint

RV-TICK-01 performed source-backed review and corrected the console work-count-only bound. A synthetic 17ms callback previously allowed subsequent queued messages to execute in the same tick; after the fix a cooperative 10ms console work deadline preserves the backlog without dequeuing unprocessed commands. The 128 request/tick cap remains, and one pending command can always progress. 74 Rust tests and a release build passed. This does NOT preempt a slow plugin callback, guarantee actual 20 TPS, or implement Folia region scheduling; see docs/IMPLEMENTATION_REFERENCE_AUDIT.md. Mandatory retrospective gate remains active.


## 2026-10-23 locked Cargo dependency provenance checkpoint

RV-LEGAL-01 added a reproducible license-declaration/checksum review gate
covering all 11 locked registry crates using standard Python and the pinned
offline Rust 1.90 Cargo metadata. Third-party notice table and reviewed
JSON snapshot are committed for reproducibility; three independent drift
tests pass. This catches metadata changes but is NOT complete legal
clearance or a formal redistribution SBOM. Per-file license texts, generated
assets, source distribution and network-source offer remain blockers.
The Phase -1 retrospective approval gate remains active. Evidence in
docs/IMPLEMENTATION_REFERENCE_AUDIT.md and docs/LICENSING.md.


## 2026-10-23 shared preauth write deadline checkpoint

RV-NET-03 closed a resource-limit asymmetry: an absolute three-second read budget existed while each outgoing write received a fresh three-second SO_SNDTIMEO. A first-party DeadlineWriter now enforces the remaining shared monotonic deadline on status replies and rejected login responses, with the narrower post-status Ping grace applied to its Pong. Three new TCP tests cover a past deadline, valid framed output and a non-reading receiver whose blocked 16MiB send completed with timeout after approximately 250ms. Full Rust suite 77 passed. This is NOT a throughput, admission-fairness or Horizon authentication result. Source rationale and limitations in docs/IMPLEMENTATION_REFERENCE_AUDIT.md; Phase -1 review hold remains active.


## 2026-10-23 review-gate source hash integrity checkpoint

RV-LEGAL-02: the source-owner coverage check previously did not verify source content digests until full approval. A changed in-review source and a missing SHA were both falsely accepted in two failing-before tests. The normal check now compares current_sha256 against actual file bytes for every in-review/approved path and rejects extraneous entries. Pending reviews are allowed to lack hashes. Three isolated Python tests cover both failures and the valid baseline; the required 9 subsystem reviews remain unapproved. The full pre-feature retrospective hold has not changed. Details: docs/IMPLEMENTATION_REFERENCE_AUDIT.md.


## 2026-10-23 per-source preauth concurrency fairness checkpoint

RV-NET-04 source-reviewed a gap in the global-only (64 by default) handshake permit guard: a single peer address could occupy every slot with pending handshakes. The revised permit tracker atomically enforces both a global cap and --max-per-ip (default 8) with IPv4-mapped IPv6 normalization, cleanup on RAII drop and bounded hash-map ownership. Three new unit regressions bring Rust total to 80. A separate release-server TCP fixture proved one held 127.0.0.1 connection rejects another from that address while status from 127.0.0.2 succeeds and the first address regains access after release. This is NOT connection rate limiting, authenticated identity or proof against distributed attacks; NAT/proxy use may require operator tuning. Phase -1 full review hold remains active. Evidence, sources and tradeoffs are in docs/IMPLEMENTATION_REFERENCE_AUDIT.md.


## 2026-10-23 default per-IP admission saturation measurement

RV-NET-05: After recording pinned FerrumC/Horizon/SteelMC comparisons in the existing reference audit, ran a controlled real release server test with global admission cap 16, per-IP cap 8, and eight held 127.0.0.1 sockets. Across two trials, every extra same-source connection was refused (16/16), every distinct-source status from 127.0.0.2 succeeded under contention (48/48), and 127.0.0.1 status recovered after release (24/24). Also measured 48 unloaded comparison requests and per-phase nearest-rank p50/p95/p99. Independent-source p50 was 0.526ms and 0.551ms in the respective contended trials; this does not demonstrate a reliable speedup or wide-area load capacity. No Rust runtime changes were required; 80 unit tests still apply. Further work: many-source global saturation and backlog pressure, NAT/proxy testing, sustained concurrency, cancellation and authenticated traffic. See docs/IMPLEMENTATION_REFERENCE_AUDIT.md and docs/NETWORK_OPTIMIZATION_2026-10-23.md. Phase -1 gate active.


## 2026-10-23 permission lookup key memory guard

RV-PERM-04 reproduced an in-process cache exhaustion class in the existing native permission evaluator. Administrative persistence validates permission/node/world identifiers at <=128 bytes, but resolve() previously stored arbitrary-length lookups into its 8192-entry cache. The new guard returns Undefined for malformed node/world parameters before allocating the cache key; regression tests verify zero cache entries for a 256KiB lookup and continued resolution for valid 128-byte names. Two added tests bring Rust suite from 80 to 82. The fix is a bounded in-process API, NOT authenticated remote input handling or LuckPerms parity. Details in docs/IMPLEMENTATION_REFERENCE_AUDIT.md; Phase -1 source review remains active.
