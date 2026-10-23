# Implementation review gate and retrospective audit

Status: **ACTIVE HOLD (2026-10-23)**. Baseline HEAD: ab02a9644ba7679985f4aef839490f20fbfc39f1.
Applies to ALL previously implemented code and ALL future implementation, optimization, refactor, and imported code.

The master roadmap is docs/ROADMAP.md. This file owns review evidence; it is not a competing roadmap. Previously published research reports are inputs, not automatic approval.

## Mandatory BEFORE implementation

1. Identify affected subsystem(s), acceptance tests, constraints, version/edition, relevant current code, HEAD and working tree.
2. Inspect primary source implementations at pinned commits, not only README claims; compare **all materially relevant** reference servers in the pool below. For sources that are irrelevant, say why; do not perform redundant irrelevant research.
3. Record exact URLs, commit SHAs, file/function names, algorithms, data structures, concurrency/locking, cache/allocations, errors and compatibility constraints for each relevant implementation. Explicitly distinguish observed source behavior, upstream benchmark claims, local experiments, and proposed hypotheses.
4. Compare current solution, upstream implementation and at least one reasonable alternative (where alternatives are meaningful). Choose based on correctness, security, parity, resource constraints, maintenance, measured performance and license obligations.
5. BEFORE code changes, record design choice, test/benchmark methodology, possible rollback, likely failure cases, license/provenance classification. A missing comparison is a blocker, not approval to skip research.
6. Make a limited, recoverable implementation. Preserve changes from other contributors. If importing source, finish per-file MIT/GPLv3/AGPLv3/dependency/asset audit first.

## Mandatory AFTER implementation / BEFORE next milestone

7. Run applicable reproducible formatting/build, unit/integration/regression, differential-parity, adverse-input/security, concurrency, live environment and performance tests. Measure before/after with identical workloads/hardware; never say faster based on intuition.
8. Review the implementation against alternatives and relevant upstream reference again: correct behavior, API compatibility, error/scheduling semantics, resource bounds, data races, licensing, provenance and performance. Record outcomes plus uncovered cases.
9. Record exact changed files, source digests, evidence links, test commands and results, remaining risks and why the selected solution is best supported for its stated use case. A green unit suite alone does not clear this step.
10. Only mark a review APPROVED if these criteria are evidenced; otherwise keep PENDING or IN_REVIEW and block unrelated new features. Changed source invalidates prior approval. A minimal security hotfix may be reviewed and verified as the focused next action, but it cannot bypass the gate.

## Reference pool, to consider per applicable subsystem

| Source | Pinned primary reference | Relevant questions |
| --- | --- | --- |
| Purpur/Paper | https://github.com/PurpurMC/Purpur/tree/5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0 and https://docs.papermc.io/paper/dev/command-api/basics/introduction/ | vanilla/configuration, command Brigadier, Bukkit/Paper APIs, events |
| Folia | https://github.com/PaperMC/Folia/tree/8cffa9059ad6bbc2b28510c38ed61a51098d4971 | region ownership, entity/global schedulers, plugin thread safety |
| SteelMC | https://github.com/Steel-Foundation/SteelMC/tree/dc8eaba7ff21c348226a05741c2fb9809a8ca90d | Rust command arena/queue, worldgen, scheduling, permissions |
| FerrumC | https://github.com/ferrumc-rs/ferrumc/tree/51830e99675deae1f6cdbb62bf5d32b3d25c1c76 | networking, Rust storage, command graphs, game-data ownership |
| Horizon | https://github.com/ludovic111/horizon/tree/e325fa65878071d5c9076e908e9a4adb258b3313 | protocol 777 codec, login/config/play, mod host and client validation |
| LuckPerms | https://github.com/LuckPerms/LuckPerms/tree/b93a4d7b24fbcb97170e355d651984493ab64e79 | tri-state, contexts, graph precedence, permission cache/storage |
| EssentialsX | https://github.com/EssentialsX/Essentials/tree/e70bdb817c758474acb3a23a37cf961b2f716a50 | utility service behavior, home/warp, teleport, player data and events |
| Specialized networking | https://github.com/astei/krypton ; https://github.com/vgskye/e4mc-minecraft ; local PulseNet research | compression, connection relaying, batching, backpressure. Verify current source and license before any reuse |

Reference applicability is scoped: review networking sources for transport, permission sources for permission decisions, worldgen sources for worldgen, etc. Irrelevance MUST be stated, not silently skipped. Re-pin for new work if upstream changes materially.

## Retrospective audit inventory — no approvals granted yet

All rows were implemented before adoption of this mandatory gate. Their presence and 48 passing tests DO NOT prove upstream parity or global optimality.

| ID | Code in current checkout | Relevant reference comparisons | Missing evidence / risks | Status |
| --- | --- | --- | --- | --- |
| REV-NET | src/network.rs, src/protocol.rs | Horizon, FerrumC, SteelMC, Paper/Purpur; specialized networking if useful | one thread per active client, per-read rather than total deadline, no auth, packet fuzz/real load | PENDING |
| REV-STATUS | status and ping in src/main.rs | Horizon, Paper/Purpur | static 0 online, incomplete malformed-status fixtures and status compatibility | PENDING |
| REV-CMD | src/command.rs, src/command/tree.rs | Paper/Purpur Brigadier, SteelMC, FerrumC, LuckPerms | no wire command tree, selectors, suggestions or differential parity | PENDING |
| REV-PERM | src/builtin/permissions.rs | LuckPerms, Paper, SteelMC | incomplete group/weight/contexts/meta semantics, simplistic full-cache eviction; no throughput benchmark | PENDING |
| REV-UTIL | src/builtin/utilities.rs, src/builtin/mod.rs | EssentialsX, Paper/Purpur, Folia | player names not authenticated UUIDs, no live teleport, events/cooldowns/world ownership | PENDING |
| REV-STORAGE | native JSON stores | LuckPerms, EssentialsX, SteelMC/FerrumC | fixed temp-path collision under multiple writers, directory fsync/migration/large datasets unverified | PENDING |
| REV-PLUG | src/native.rs, src/plugins.rs, Java bridge | Horizon Wasmtime, Paper/Purpur, Folia, EssentialsX/LuckPerms | native ABI unsandboxed, CStr pointer read before max-length check, JVM wait may hang, no real Bukkit API | PENDING |
| REV-TICK | src/main.rs scheduler and commands | Folia, SteelMC, Paper/Purpur | 50ms tick is a target, not measured TPS; no simulation action ownership | PENDING |
| REV-LEGAL | Cargo.lock/LICENSE/notices | all upstream source licenses | no full dependency SBOM/distribution test; AGPL source-offer release mechanism pending | PENDING |

## Initial findings and focus before any new feature

- HIGH, REV-PLUG: src/native.rs calls CStr::from_ptr(ptr).to_bytes() before checking 8,192 bytes. Its pointer is trusted/unsafe but a malformed plugin can trigger unbounded reads; replace with a length-delimited, documented ABI and test error cases before stabilizing plugin API.
- HIGH, REV-NET: fail closed on unauthenticated sessions, complete connection deadlines and cancellation; version 777 + a brand field is not Horizon-only proof.
- HIGH, REV-PERM/REV-UTIL: identity is name-oriented and semantics diverge from full LuckPerms/EssentialsX behavior; require UUID, contextual parity and persistence tests.
- MEDIUM, REV-STORAGE: full JSON state copy-and-rewrite for every write, fixed temporary path and missing directory durability audit; test crash/restart and concurrent storage before treating as production.
- MEDIUM, REV-PLUG: Java child wait can block indefinitely when plugin misbehaves; no Java plugin API interoperability established.
- NEEDS BENCHMARK: last status RTT improvement pertains to one localhost workload, not total throughput or Minecraft gameplay. No comparative performance evidence exists for permission, plugin, utility or worldgen paths.

## Baseline validation and evidence confidence

Frozen base ab02a96: git tree clean; Rust 1.90 cargo test --all-targets --quiet: 48 passed, 0 failed; cargo fmt --all -- --check passed. Prior docs/REFERENCE_ARCHITECTURE_RESEARCH_2026-10-23.md and docs/NETWORK_OPTIMIZATION_2026-10-23.md are partial primary-source research/benchmark evidence, not completed code-parity reviews.

To approve any row: record specific source-file references and versions, alternatives, acceptance and measured/verified tests, license decision, unresolved risks and SHA256 of reviewed source files in the ledger. A digest change makes the approval stale. Do not call any implementation 'the fastest' or 'best possible' without workload-specific comparable evidence.

### Resume priority

Retrospective source comparison and critical correctness/security remediation, in this order: REV-PLUG unsafe metadata API; REV-NET connection/handshake deadlines; REV-PERM identity and semantics; REV-STORAGE atomicity; REV-CMD and REV-TICK. Maintain the user requirement that native EssentialsX/LuckPerms-like services ship within Rust without mandatory Java JARs. No major new feature until retrospective review gate is cleared.


## REV-PLUG targeted pre-implementation review — 2026-10-23

Scope: src/native.rs, examples/rust-plugin/src/lib.rs, src/plugins.rs and trusted Rust native plugin manifest metadata (including bounded Java sidecar shutdown). This is focused security remediation, not full plugin API approval.

Source-grounded alternatives: Horizon e325fa6 crates/horizon-mods/src/runtime.rs uses Wasmtime component memory/fuel/host quotas, avoiding direct host dereference of untrusted module pointers. Paper plugin.yml and plugin lifecycle docs require JVM JavaPlugin entry points/classloaders; they are not a native C ABI. FerrumC 51830e9 src/lib/plugins/src/lib.rs is a placeholder add-function example, so is not a production ABI source. SteelMC dc8eaba steel-core/src/command/mod.rs uses a structured command registry, not a native pointer metadata API. Folia is relevant for future plugin thread ownership, not manifest byte buffer. EssentialsX/LuckPerms require Java APIs and do not solve this native ABI. Krypton/e4mc/PulseNet are transport-specific and inapplicable here.

Known fault: host CStr::from_ptr returned by optional horizon_plugin_commands() scans for NUL before checking 8192 bytes; no length bound on dereference. Alternatives: (A) scan only up to max but still trust a foreign pointer, (B) return pointer plus length and still dereference foreign data, (C) selected optional horizon_plugin_commands_v2(buf:*mut u8,cap:usize)->usize with host-owned output buffer (8192 bytes), explicit returned length and UTF-8 validation. Event callback ABI v1 remains. Old command-manifest v1 symbol is ignored (legacy plugin can load but does not register its command), a documented compatibility change. Host-owned bytes remove this specific blind scan; native dylibs remain fully privileged and not sandboxed.

Java child process: blocking Child::wait after STOP can hang if onDisable misbehaves. Choose graceful closing of stdin to send EOF, a short try_wait grace period, then kill and reap, over either indefinite wait or unconditional termination. Cross-process STOP delivery and subprocess ownership are tested separately.

Verification plan: buffer length/UTF-8/binary/zero-length tests, real Rust plugin command dispatch with v2, absent v2 legacy no-command behavior, separate Java normal lifecycle plus intentionally hung onDisable termination, all prior Rust tests, rustfmt, release, standalone smoke. Original AGPL Rust changes; no copied upstream code. This is not performance benchmarking. Roll back the scoped patch on regression.

Primary references:
- https://github.com/ludovic111/horizon/blob/e325fa65878071d5c9076e908e9a4adb258b3313/crates/horizon-mods/src/runtime.rs
- https://docs.papermc.io/paper/dev/how-do-plugins-work/
- https://docs.papermc.io/paper/dev/plugin-yml/
- https://github.com/ferrumc-rs/ferrumc/blob/51830e99675deae1f6cdbb62bf5d32b3d25c1c76/src/lib/plugins/src/lib.rs
- https://github.com/Steel-Foundation/SteelMC/blob/dc8eaba7ff21c348226a05741c2fb9809a8ca90d/steel-core/src/command/mod.rs


## REV-NET scoped pre-implementation review — total pre-authentication deadline (2026-10-23)

Scope: src/network.rs and pre-auth packet reads in src/main.rs. Current behavior sets a 3-second socket read timeout once; each new successful single-byte read restarts the operating system wait. A peer can slowly transmit a valid frame over much longer than three seconds and hold a bounded but valuable worker slot.

Primary implementation references: FerrumC 51830e9 src/lib/net/src/connection.rs declares MAX_HANDSHAKE_TIMEOUT=10 seconds and wraps handshake with tokio::time::timeout; the returned deadline covers the handshake future, unlike per-byte waits. Horizon e325fa6 crates/horizon-protocol/src/connection.rs separates stream packet reads and uses timeout-aware TCP/in-memory streams; its individual Pipe.read timeout should not be mistaken for a global connection admission budget. SteelMC's bounded command queue provides an analogous resource-budget pattern but no direct pre-auth socket deadline. Paper/Purpur and Folia are semantically relevant to handshake correctness and thread ownership, respectively, but not this blocking Rust TCP timeout implementation. Krypton/e4mc/PulseNet packet batching is not relevant to a slow-drip handshake and no source is copied.

Alternatives: migrate entire proof-of-concept to Tokio (complex, inappropriate for a 48/49-test pre-playable server), force a fixed timeout per read (current exploitable behavior), or wrap TCP Read calls with a single monotonic deadline and recompute remaining SO_RCVTIMEO per read. Chosen: small deadline-aware Reader adapter, one absolute deadline shared across handshake, status, optional ping and login hello, retaining existing framing/resource limits and blocking thread model. The loop refuses further reads once expired, even when a client sends a byte before every old per-read timeout. This is a bounded admission improvement, not a cryptographic authentication implementation or proof of high-throughput network resilience.

Acceptance: deterministic deadline-expired unit check, real TCP slow-drip test, unchanged status/login regression tests, all Rust suite, release build, reproducible status performance sanity. Work remains on total write deadlines, cancellation, per-IP admission, reactor transport and authenticated Horizon-only negotiation. First-party AGPL changes, no upstream code imported.


## 2026-10-23 targeted remediation verification — REV-PLUG and REV-NET

**Implementation and outcomes**
- REV-PLUG: optional native manifest export moved from pointer-to-NUL (v1 manifest) to caller-owned buffer plus reported length (v2). The host checks 0 < length <= 8192, embedded NUL and UTF-8 before converting to a Rust String; legacy v1 manifest symbols are no longer consumed, but core native event ABI v1 is unchanged. Original AGPL source only; no imported upstream implementation. Evidence: src/native.rs, examples/rust-plugin/src/lib.rs, native::tests::manifest_v2_bounds_utf8_and_binary_validation.
- REV-PLUG: Java bridge stdin is dropped to send EOF, granting 1.5 seconds for the child to disable plugins before termination. Evidence: src/plugins.rs and scripts/test_plugin_lifecycle.py. The fixture compiling a real Java 21 JAR completed normal shutdown and enabled/disabled; a deliberately hanging onDisable was terminated after its grace interval (observed end-to-end elapsed 3.77 seconds, including deliberate initialization sleep).
- REV-NET: all preauth read operations now share one three-second absolute monotonic deadline, recalculating socket timeout per Read call. Against a slow drip client sending one frame body byte every 250ms, the 430ms test deadline rejected before full frame ingestion. Evidence: src/network.rs DeadlineReader, src/main.rs serve, tests expired_deadline_never_reads_available_bytes and slow_drip_has_global_deadline_not_per_byte_timeout. This is source-inspired by FerrumC's whole-handshake future timeout; it is not a full asynchronous transport port.

**Actual verification**: cargo +1.90.0 test --all-targets passed 51/51 tests (48 original + 3 new tests), cargo +1.90.0 fmt --all -- --check passed, both server and sample native Rust plugin release builds passed. python3 scripts/test_plugin_lifecycle.py passed normal and hung JAR fixtures. python3 scripts/reference_review_gate.py --check passed with zero uncovered source files. The legacy native manifest is intentionally incompatible for command registration; it must be rebuilt to ABI v2. The Java and native lifecycle engine are still experimental, not full Bukkit compatibility or a native security sandbox.

**Performance retest, status-only**: python3 scripts/bench_status.py target/release/purpur-horizon, localhost sequential 120 plus 240 requests at eight-way concurrency, recorded two independent current runs: sequential p50 0.346ms then 0.129ms; concurrent p95 2.062ms then 1.787ms. Earlier accepted optimized build reported sequential p50 0.117-0.128ms and concurrent p95 1.522-1.750ms. Large variance on the same host indicates scheduling/background-load sensitivity. No throughput gain or global fastest claim. No real Horizon client, authentication, gameplay TPS or stress benchmark completed.

**Remaining work and gate status**: REV-PLUG IN_REVIEW: untrusted native DLLs still execute arbitrary code; manifest writer can corrupt caller memory if malicious; Java Bukkit/Paper APIs, WIT sandbox migration, Java process-tree control, plugin scheduler ordering, and a complete ABI conformance suite remain. REV-NET IN_REVIEW: no actual Horizon login, full write-side deadlines, total transaction/accounting limits, per-IP concurrency, shutdown cancellation, fuzz corpus or large-scale traffic evidence. Both stay unapproved. RETROSPECTIVE REVIEW HOLD REMAINS ACTIVE.

**Compared implementation options after patch**: Horizon WIT/Wasmtime is preferable for untrusted plugins when implemented; the host-owned manifest buffer is a small improvement only to the explicitly trusted native extension proof of concept. FerrumC's Tokio timeout wraps whole handshake futures; our small blocking reader provides similar total-read-window semantics without bringing the unneeded async dependency into the pre-playable server. Full transition to async reactor needs measured player traffic and packet correctness.


## REV-STORAGE targeted pre-implementation comparison (2026-10-23)

Current JSON writer in src/builtin/permissions.rs::atomic_json has fixed path json.tmp and truncates it on every call. Two independent services can overwrite a shared temporary file, and successful commits can exceed the MAX_SIZE=2MiB limit enforced only on restart. No parent directory fsync confirms rename durability. The current code clones full maps and rewrites them on every update.

Pinned primary sources:
- LuckPerms b93a4d7 common/.../storage/implementation/file/SeparatedConfigurateStorage.java: per-path ReentrantLock, separate user/group/track files and watcher invalidation.
- LuckPerms b93a4d7 common/.../storage/StorageFactory.java: selectable SQLite/H2/SQL/NoSQL/file backends, unlike our monolithic snapshot.
- EssentialsX e70bdb8 Essentials/.../UserData.java: UUID-named per-player YAML, explicit blocking saves on cleanup and config reload. ManagedFile.java manages versioned packaged resources, not a transactional player store.
- FerrumC 51830e9 src/lib/storage/src/lmdb.rs: LMDB MVCC, parallel read transactions and one serialized writer. Schema/dependency migration is future work.
- SteelMC dc8eaba steel-core/src/chunk_saver/region_manager.rs: async region handles and prepared per-chunk writes, on-demand data and cached 8-KiB headers, not a drop-in permission backend.
- Paper/Purpur/Folia help define ownership and durability expectations, not the implementation of this Rust JSON writer. Horizon, Krypton, e4mc and PulseNet are unrelated to this file persistence change.

Options: A retain fixed-temp and process convention (lost updates possible); B unique temps alone (prevents clobbered temp but not concurrent lost updates); C immediately use SQLite/LMDB (much larger migration/unbenchmarked cost); D chosen single-writer JSON safety gate: deterministic advisory lockfile owned for each service lifetime; unique create_new temp; precommit serialized size cap; sync temp and parent directory. This preserves v1 schema, fails fast on second active store, prevents temp truncation, and is proportional to current prototype stage.

Acceptance: unchanged schema and restarts, fail second open while writer active, no successful oversized snapshots, fail-closed startup on corruption, permission/utilities regressions, release build and no-JVM smoke. Still not a cross-host transactional database, optimized throughput or full LuckPerms/EssentialsX storage parity. First-party AGPL code only, no upstream imports. Persisted lockfiles are expected: OS releases locks on process exit.


## REV-STORAGE focused post-implementation verification — 2026-10-23

Files changed: src/builtin/permissions.rs (shared writer, StoreLock), src/builtin/utilities.rs (store lock), src/builtin/mod.rs (restart integration), scripts/test_single_writer.py. No third-party source copied or translated.

Result versus options: kept first-party Rust snapshot storage with v1 JSON schema but added lifetime exclusive advisory .json.lock handles (one concurrent process allowed per logical store), unique PID/monotonic-counter temporary filename using create_new instead of clobbering json.tmp, enforced the existing 2 MiB size cap on serialized output, called sync_all on the temporary file before rename and sync_all on its parent directory after replacement on Unix. The global full-map clone/rewrite cost remains and is not proven optimal; future SQLite/LMDB migration requires measured workload and migration tests.

Tests: 54/54 Rust unit/integration tests passed (51 baseline + three new tests); deliberately invalid JSON test was corrected to close the writer first, ensuring the corruption parser rather than advisory lock causes rejection. Native permissions, utilities and command integration restart tests still pass. scripts/test_single_writer.py launched two independent server processes against the same data directory: second process exited with lock failure; after first server stopped, re-opening succeeded and both permission grant and saved home remained. Existing Java normal/hung lifecycle fixtures continued passing; release compilation and formatting passed.

Unverified: abrupt power-cut on actual filesystem, advisory lock behavior on remote/uncooperative clients or network mounts, operating-system-specific filesystem semantics, multi-process DB transaction isolation (intentionally unsupported), data volume/throughput scaling, full LuckPerms/EssentialsX semantic/data compatibility, and authenticated-UUID identity. In particular, if parent-directory sync fails AFTER a successful rename, the caller currently sees an I/O error while the on-disk file may already be updated; in-memory state is not published. This requires an explicit ambiguous-durability/error recovery policy before production.

REV-STORAGE is IN_REVIEW rather than approved. The original source comparisons, alternatives, acceptance plan and remaining risk are recorded above. The source hash ledger is updated, and the review hold remains active.


## REV-STORAGE-02 pre-implementation comparison: ambiguous post-rename failure (2026-10-23)

Initial defect: Rust atomic_json syncs a temporary file, renames over the current snapshot and syncs the parent directory. A failing post-rename directory sync_all returns error, so NativePermissions::commit and NativeUtilities::commit do not publish the already-renamed next snapshot in memory. Subsequent mutations can overwrite the newer disk snapshot with stale state. This is a state-consistency gap, not a reproduced power-failure claim.

Relevant pinned upstream primary reference findings, previously inspected for REV-STORAGE-01:
- LuckPerms b93a4d7 SeparatedConfigurateStorage.java uses per-file locks and watcher invalidation for disk-backed users/groups; StorageFactory.java provides SQLite, SQL and file modes. Ambiguous writes must not be ordinary pre-write failures.
- FerrumC 51830e9 src/lib/storage/src/lmdb.rs uses LMDB write transactions with explicit commit boundaries. Migrating v1 JSON now is much larger and unrelated.
- EssentialsX e70bdb8 UserData.java uses UUID player data and explicit blocking saves; not a transactional JSON strategy.
- SteelMC dc8eaba region_manager.rs separates prepared chunks and async region commits; not a drop-in permission data store.
- Paper/Purpur/Folia scheduling and Horizon networking are not directly relevant to a Rust fsync-after-rename error. Other reviews cover their applicability. Krypton/e4mc/PulseNet are transport-only.

Alternatives: (A) swallow fsync failure (false durability assertion), (B) publish next state but return error (ambiguous retry), (C) rollback rename without durable backup (unreliable), (D) selected: return a phase-tagged write error, poison the live store after any post-replace failure, clear/fail closed permission lookups and suppress stale utilities, prevent further commits, require closing and reopening under the existing writer OS file lock. Errors before rename leave memory intact and subsequent writes operational.

Test criteria: deterministic fault injection before/after rename; disk contains next snapshot after injected directory failure; poisoned permission checks are Undefined and additional writes reject; utility reads hide stale data; successful fresh reopen and v1 format, all baseline Rust regressions, no-JVM/JVM smoke and build. Not a power-loss reproduction, SQLite transactional replacement, new benchmark or full LuckPerms parity. Changes are original AGPL Rust, no third-party code imported. Keep audit in-review until full scope is verified.


## REV-STORAGE-02 post-implementation verification (2026-10-23)

The targeted error-phase repair is implemented in src/builtin/permissions.rs and src/builtin/utilities.rs. The shared atomic JSON writer returns a structured phase status identifying whether the rename completed. Directory sync failures AFTER rename poison the store; additional commits fail until the service is dropped and reopened under the same exclusive lock. A permission store invalidates its cache and returns Undefined for all checks when poisoned, preventing use of a stale in-memory authorization. A utility store hides stale home/warp lookups and lists while poisoned. Ordinary errors before rename leave the store writable and unchanged. The JSON v1 schema, atomic replacement and size bound remain unchanged.

Direct verification, not simulated live power failure: two new permission tests inject a before-rename failure and a directory-sync error after the actual rename, exercise a previously warmed Allow result, assert no further writes, inspect exact disk bytes and reopen to the newer snapshot. A utility test injects the same post-rename failure, asserts stale warp lookups/lists cannot be used or overwritten, and reopens the newer disk data. Rust 1.90 test suite is 57/57 passed, formatting and release build passed; the updated real-process persistence smoke verifies second-writer rejection, normal restart, SIGKILL/restart and surviving permissions/homes. Java normal and hung-plugin shutdown fixture also remained green. No new third-party source code was copied.

The chosen behavior rejects live state mutations rather than making a false rollback/durability guarantee, unlike swallowing the error or returning an ambiguous failed commit while retaining write access. It is more conservative than both a full database transaction and silently adopting the renamed in-memory data. The available upstream LuckPerms/FerrumC/EssentialsX/SteelMC file references, alternatives and license analysis are detailed in the preceding pre-implementation section.

REV-STORAGE remains IN_REVIEW. Unverified: real disk/power failure, filesystem-specific directory sync semantics, cross-filesystem rename, NFS/advisory lock cooperation, transactional multi-store consistency, high-volume read/write benchmarks, schema and UUID migrations, SQLite/LMDB data model choice. This patch is not production-grade durability certification or permission semantic parity.


## RV-PERM-01 pre-implementation decision: legacy subject case ambiguity (2026-10-23)

The native v1 permission data store keys subject names literally. Console grants for Alex become user:Alex; later checks for alex start from user:alex. A case change can therefore bypass or lose a previously configured deny/allow or group membership. The user remains **not authenticated**; this is a local consistency bug only, not player identity security.

Source comparison:
- LuckPerms b93a4d7 PermissionCalculatorBase.java lowercases permission nodes before its ordered processor chain; AbstractCachedDataManager.java caches per QueryOptions and explicitly invalidates/reloads; Graph.java allows configurable graph traversal. None of those are represented by this prototype's fixed specificity tuple.
- LuckPerms b93a4d7 StorageFactory.java and SeparatedConfigurateStorage.java use identity-oriented user data, per-path locking and watcher invalidation; do not treat mixed-case display names as separate authenticated permission owners.
- EssentialsX e70bdb8 UserData.java creates per-player configuration files by base.getUniqueId(), preserving usernames only as metadata. This highlights the eventual need for authenticated UUID principals, not authority from display spelling.
- SteelMC dc8eaba steel-core/src/permission/manager.rs uses validated group config snapshots and persistence-first updates and allows typed assigned groups; does not imply the present username map is secure.
- Paper/Purpur and Folia inform the future authenticated player/permission and scheduler handoff but do not specify this Rust v1 JSON key normalization. FerrumC transport/network, Horizon packets and specialized networking sources are not relevant to this narrow identity-key migration.

Alternatives: reject all legacy case-distinct names (breaks existing stores), rely on case-sensitive names (bypass bug), silently merge case-colliding legacy grant maps (could produce escalated access), or chosen: canonical lowercase ASCII user/group subject keys at write/read, normalize unambiguous historic v1 keys in memory when opened, reject duplicate case aliases as corrupt/ambiguous, and let the next successfully persisted mutation write the normalized form. Do not change world-context names, permission precedence, or data version. This does NOT replace authenticated UUID identities or claim LuckPerms parity.

Acceptance: case-insensitive direct and inherited grants/denies; persisted unambiguous mixed-case historical file can be read without data loss; colliding user:Alex and user:alex entries cause fail-closed startup rather than merging; fresh serialized grants are normalized; existing tests, release and smoke unchanged. No source copied (first-party AGPL Rust). Full semantic REV-PERM remains IN_REVIEW.


## RV-PERM-01 post-implementation review: canonical names and legacy migration (2026-10-23)

Changed src/builtin/permissions.rs only. The user-entered case-sensitive key difference between config_subject and resolve is fixed by ASCII-lowercasing legacy subject and group keys on grant, direct query, parent insertion and removal; the cache key is also lowercase. During open, the existing JSON v1 grants/parents maps are canonicalized in memory, nodes lowercased, and duplicate parent links collapsed. If two pre-existing keys map to one subject (e.g. user:Alex and user:alex), startup rejects with InvalidData rather than choosing or merging opposite grants. Legacy unambiguous JSON is read safely and next successful save writes canonical keys. World context strings are deliberately left as provided because dimension/world names have their own semantics.

Relevant upstream comparison after code: LuckPerms PermissionCalculatorBase.java lowercases permission nodes before processing and caches results; AbstractCachedDataManager.java uses context QueryOptions, not our world-only key. LuckPerms User/graph storage and EssentialsX UserData.java (UUID-named player files) support future stable UUID identities. SteelMC permission/manager.rs validates group snapshots and persists before publication. This fix is deliberately smaller than importing their full semantics: no UUID authentication or true LuckPerms group-weight/context processor parity. Purpur/Paper lifecycle belongs to an eventual live-player identity API rather than this console JSON compatibility patch.

Verification: three new Rust tests check mixed-case direct Deny/inherited Allow/group removal, file contents normalized, historic mixed-case grants and group parents preserved over a new commit, and conflicting legacy subject aliases rejected on open; all 60 Rust tests passed. The independent release binary also passed scripts/test_single_writer.py with uppercase ALICE grant and lowercase alice lookup across clean and SIGKILL restarts; native/Java lifecycle regression passed. No third-party source copied; no timing or throughput improvement claimed. Fail-closed ambiguous rename behavior from RV-STORAGE-02 remains tested.

Remaining REV-PERM blockers: enforce authenticated UUID principals before real player admission, model LuckPerms QueryOptions and ordered processor semantics, default groups/weights/tracks/metastacking, precise user/group precedence, expiring inheritance, online/offline migrations and measured caching. REV-PERM is IN_REVIEW, not approved. REV-STORAGE remains IN_REVIEW because data format was preserved but complete crash/scale/oracle validation is pending.


## RV-PERM-02 — group traversal saturation: pre-implementation review (2026-10-23)

Scope and defect: src/builtin/permissions.rs::resolve breadth-first walks subject/group ancestry and caps unique visited nodes at MAX_SUBJECTS=128. Current condition skips a newly visited 129th subject but may return an Allow accumulated from an incomplete graph. A valid v1 JSON file can contain 128 direct group parents and a user root, exceeding the traversal bound without violating the stored schema.

Source comparison, inspected before change:
- LuckPerms b93a4d7 common/src/main/java/me/lucko/luckperms/common/graph/TraversalAlgorithm.java implements BFS/DFS graph traversal with a deduplicating visited set; PermissionCalculatorBase.java has a tri-state ordered processor chain, NOT the same as our fixed specificity rank. The artificial 128-visit limit is not claimed as LuckPerms behavior.
- SteelMC dc8eaba steel-core/src/permission/groups.rs::from_config validates group inheritance upfront and effective_permissions deduplicates inherited groups using a BTreeSet. SteelMC also has default groups and priorities absent here; its manager.rs publishes validated snapshots.
- Paper/Purpur live permission contracts and Folia entity ownership apply to later authenticated-player work, not this local Rust 128-entry traversal contract. EssentialsX delegates to Bukkit/Paper permissions, and FerrumC transport/storage, Horizon client and specialized networking references are not applicable.

Alternatives: (A) keep partial Allow on overflow (unsafe); (B) increase cap (same latent partial bug); (C) validate all possible ancestry closure on each write (more costly and migration work); (D) CHOSEN: retain the bounded traversal and return Undefined (deny) on any newly visited subject that exceeds the limit. In-range behavior and existing cache remain unchanged. Later evaluate graph validation/cap with benchmarks and a semantic oracle. All changes first-party AGPL Rust without upstream code copied.

Test acceptance: first reproduce overflow Allow using 128 schema-valid direct parents, validate Allow with 127 direct parents, Undefined when adding the 128th (129 distinct including root), and recover Allow after its removal with cache invalidation; full Rust suite, release and real-process smoke. No invented LuckPerms parity or throughput claims.


## RV-PERM-02 additional boundary: persisted inheritance namespace (pre-implementation)

Existing v1 schema permits parent link strings passing only valid_id, including user:admin. The legitimate add_parent method writes group:NAME targets, and inherited group traversal reads all targets without checking namespace. A manually modified but otherwise syntactically valid JSON snapshot can therefore make one user inherit another user's rule set, which is outside the declared group-only semantics. Likewise a bare or empty user:/group: subject key is currently valid_id but is never intentionally emitted by normal writes.

Pinned source reuse: LuckPerms b93a4d7 graph TraversalAlgorithm operates on already-typed graph nodes; SteelMC dc8eaba groups.rs validates named groups and inheritance before publication, including missing groups/cycles. The Rust prototype does not need to implement complete group existence or priority today, but must avoid crossing its explicit user/group namespace boundary. Paper/Purpur and Folia live-player semantics remain future tasks; unrelated storage/network references are inapplicable.

Alternatives: ignore malformed persisted edges (could conceal misconfiguration and existing grants), follow arbitrary user:* edges (authorization escalation), migrate arbitrary bare keys to users (could merge identities), or chosen **reject invalid v1 subject keys and non-group inheritance edges during load and write validation**. Preserve valid historical mixed-case names, including case-normalization repair. No new schema or imported third-party code. Acceptance: reproduce unsafe cross-user inheritance before patch, reject it on open, reject empty/bare namespace keys, preserve legitimate group inheritance/restart, all regression/integration tests. Never claim complete LuckPerms parity.


## RV-PERM-02 post-implementation verification — saturation and identity boundaries (2026-10-23)

Baseline: 60 Rust tests on clean cc802446cb677f3db7e0bc1d1f2656e8434e4d16. A new test failed against the old code, observing Allow where a 129-subject group traversal (valid 128 direct links plus user root) should have returned Undefined. A separate new test failed against the old code because a manually edited JSON v1 snapshot allowed a parent edge from user:alex to user:admin and accepted missing/empty subject namespaces.

Implementation: src/builtin/permissions.rs now returns Undefined immediately when visiting a 129th distinct reachable subject rather than preserving a partial positive decision. The existing 128-subject cap, BFS semantics and v1 schema are unchanged. Stored grant/parent keys must have a nonempty user: or group: namespace, and inheritance edges must use a nonempty group: target. Invalid persisted values are rejected as InvalidData at open. Original first-party AGPL-3.0-or-later Rust; no source transplanted from LuckPerms or SteelMC.

Verification: two distinct regressions fail on previous code, then succeed after remediation. The boundary test verifies Allow with exactly 128 reachable subjects, Undefined at 129 even with a preexisting direct Allow, and Allow after removing the excess group with cache invalidation. The schema test verifies fail-closed rejection for user-to-user inheritance and empty/bare subject names. Full Rust 1.90 unit suite: 62/62 passed; release build passed. Neither a LuckPerms semantic oracle nor performance benchmark was executed. In particular LuckPerms's configurable BFS/DFS graph traversal and ordered permission processors, and SteelMC's startup validation/priority model, are more complete. This fix enforces local authorization safety under the current capped implementation; it does not establish equivalence to either server.

Remaining: benchmark a validated graph/precomputed closure strategy, differential processor/context/expiry/group priority rules, authenticated UUID identity and safe future schema migrations. REV-PERM still IN_REVIEW; retrospective advancement gate remains enforced.


## RV-PERM-03 pre-implementation review: typed principal aliases and namespaces (2026-10-23)

Confirmed source-design inconsistency requiring a regression: permissions set user:alex node allow writes user:alex, but resolve("user:alex") blindly constructs user:user:alex. Similarly permissions parent alex group:staff constructs group:group:staff rather than referencing the group:staff grant. Unqualified group identities passed as user lookups must never grant group privileges.

Pinned upstream comparison: LuckPerms b93a4d7 PermissionCalculatorBase.java separates permission processing and tri-state result from typed graph identity; TraversalAlgorithm.java traverses graph nodes with visited-set deduplication, not accidental string concatenation. SteelMC dc8eaba steel-core/src/permission/groups.rs parses validated named groups, applies assigned group permissions and subject-specific overrides separately, and supplies default groups and priorities absent here. EssentialsX e70bdb8 UserData.java uses authenticated UUID-bound player files, which reinforces that these console names are not security identities. Paper/Purpur player APIs depend on validated player identities; Folia scheduler, FerrumC network/storage, Horizon login and Krypton/e4mc/PulseNet are not directly relevant to console namespace parsing. No external source reused.

Alternatives: A disallow qualified identities everywhere (breaks established group:staff mutation syntax); B keep blind string-prefix construction (inconsistent); C CHOSEN parse one optional user: or group: namespace and only accept validated nonempty ASCII name components, disallow nested colons. User checks only look up user identities, accepting bare alex and user:alex equivalently, returning Undefined for group:staff. Parent group references accept bare staff and group:staff equivalently; invalid/mismatched/nested prefixes reject rather than producing inaccessible records. Existing v1 JSON with correctly namespaced keys remains valid; corrupted nested keys are not silently merged.

Acceptance: failing-before / passing-after regression for explicit user principal and group name across native command dispatch, group spoof fail-closed, malformed namespace rejection on mutation, JSON readback and all 62 prior tests. No full LuckPerms semantic parity or benchmark speedup implied. Original AGPL Rust only; continue retrospective hold.


## RV-PERM-03 post-implementation verification: typed namespace consistency (2026-10-23)

Reproduced before the patch: the new qualified-principal unit test failed because the old resolver returned Undefined for a grant stored as user:Alex and queried as user:ALEx. The old add_parent implementation likewise prepended group: even if its group argument was already group:staff, creating a disconnected group:group:staff edge. After the patch, both bare and precisely once-qualified legacy console identities canonicalize to the same persistent subject. Group names supplied to add_parent/remove_parent accept bare or group: forms; invalid cross-namespace, nested or empty names are rejected. User-only permission checks never resolve group:staff to group grants, even though administrative grant editing can address group:staff explicitly. Existing JSON v1 stores with valid subject syntax continue loading; bad nested namespaces reject at open instead of being silently merged.

Evidence: src/builtin/permissions.rs typed_principal_names_agree_with_bare_names_and_reject_spoofed_groups; expanded corrupted v1 JSON cases; scripts/test_single_writer.py real release-binary console sequence uses qualified user/group arguments, checks direct Allow and inherited Deny after clean and SIGKILL restarts, and confirms direct group-name spoof yields Undefined. Rust test suite and all integration checks run after the fix. No external source code copied; no benchmark speedup claimed. The parser still handles only console-oriented name keys, not authenticated player UUIDs; full LuckPerms defaults, contexts, weighted inheritance and processor precedence remain absent. REV-PERM stays IN_REVIEW. This is a focused compatibility and security boundary repair, not a broad review approval.


## RV-NET-02 scoped pre-implementation review: status-only connection slot retention (2026-10-23)

Current source observation: src/main.rs::serve allocates a bounded worker permit for each accepted TCP connection. After sending the server-list status JSON, it waits for an optional status Ping request via DeadlineReader with the original global three-second admission deadline. A client that requests only status, or intentionally withholds Ping, can keep one of at most 64 admitted worker slots occupied for nearly three seconds despite already having received the answer. This limits effective new-client acceptance under status-only traffic. No player admission or performance benchmark is claimed.

Pinned primary references: Horizon e325fa65878071d5c9076e908e9a4adb258b3313 crates/horizon-protocol/src/packets/status.rs defines status request/response as a distinct exchange from PingRequest/PongResponse, with a signed 64-bit echoed ping time. The second packet is distinct, not a prerequisite to the initial status response. FerrumC 51830e99675deae1f6cdbb62bf5d32b3d25c1c76 src/lib/net/src/connection.rs bounds its whole pre-play handshake future with tokio::time::timeout and drops stalled connections to avoid resource hogging; this is a related admission-bounding principle, not an exact status ping deadline. Paper/Purpur server-list ping semantics require echoing a legitimate ping; those implementations are JVM-based and do not require retaining a one-thread Rust worker for the remaining 3 seconds. SteelMC bounded queueing inspired the existing worker cap, not ping-specific scheduling. Folia scheduling, EssentialsX/LuckPerms data, and Krypton/e4mc/PulseNet packet batching are irrelevant to this optional status ping timeout. No upstream code imported.

Alternatives: A keep 3-second ping grace (avoids early close but consumes worker slots); B do not wait at all after status (breaks status Ping/Pong); C one-second bounded optional Ping window from time status reply is written, capped by global 3-second handshake expiry (chosen). This reduces idle worker residency while allowing high-latency clients a separate 1-second opportunity to ping after status. Higher-latency/bursting links beyond this window may fail to receive Pong; do not advertise exact vanilla equivalence. D replace thread-per-connection with nonblocking event loop (larger transport rewrite requiring real load evidence). Retain strict 9-byte Ping framing and current response bytes.

Acceptance: reproduce that status-only connections currently hold a worker around three seconds, then test release within less than ~1.7 seconds while preserving direct status/Pong echo correctness and 62/63 previous tests. Benchmark with method identifying status-only worker residency, not extrapolating to TPS or internet behavior. Preserve earlier total handshake deadline and connection slot protections. Original first-party Rust code, no third-party copy. REV-NET remains IN_REVIEW.


## RV-NET-02 post-implementation verification: optional status Ping window (2026-10-23)

The failing-before test tests::status_only_peer_releases_worker_without_three_second_ping_hold held its TCP stream open after consuming server-list status and withheld Ping. The old worker completed in 3.035088 seconds, failing the less-than-1.7-second acceptance requirement. The new code in src/main.rs calculates an independent post-status Ping deadline as the minimum of the existing global three-second deadline and now plus one second. It continues to decode exactly a 9-byte serverbound Ping and echo its original signed-64-bit payload. This is an input-admission resource limit, not an async reactor or connection-rate limiter.

After patch: status-only worker completed in approximately 1.02 seconds on localhost, passing the same TCP test. The original immediate Ping/Pong regression still passed. A new TCP regression delays Ping 180 ms after status reply, then checks an unchanged negative signed-64-bit echoed payload; it passed. Full Rust suite increased from 63 to 65 tests after two status tests. Source comparison was recorded above before code. No source copied from Horizon, FerrumC, Paper or other references.

Tradeoff: clients whose status-to-ping turnaround exceeds one second may miss Pong, including extreme internet latency or pauses. The three-second global handshake remains for the first status request or login stages, and status-only connections can still consume capacity for up to about a second. No simulated wide-area latency, high-concurrency admission rate, CPU profiling or real game TPS was measured. The status-only test measures one socket's worker lifetime, not a rate/throughput distribution. Follow-up: admission pressure simulation, per-IP limits, cancellation and eventual bounded I/O reactor; consider a separately configurable ping grace based on real-client evidence. REV-NET and REV-STATUS are IN_REVIEW rather than APPROVED.


## RV-CMD-01 pre-implementation review: canonical command identity and bounded grants (2026-10-23)

Current source: src/command.rs Registry::register lowercases route keys and aliases, but retains original case in Command.name. Registration of Test:Admin thus creates test:admin route while execute returns a Core or Plugin effect carrying Test:Admin and visible advertises different text. Core handler dispatch matches lowercase strings; accepted registrations can be undeliverable. Separately src/command.rs Grants::allowed is a legacy in-memory helper (tests, NOT production NativePermissions) which truncates BFS when subjects.len() reaches 64, then evaluates grants found in the partial graph. A partial Allow cannot safely authorize commands if later group inheritance was not visited.

Pinned source comparison: SteelMC dc8eaba steel-core/src/command/brigadier/dispatcher.rs::register calls normalize and validates a literal tree before merging; its parsing and requirements use the same owned dispatcher nodes. SteelMC steel-core/src/command/protocol.rs builds command-tree/suggestion packets from visible validated nodes with caps; our graph is explicitly protocol neutral, not yet a Minecraft Commands packet. LuckPerms b93a4d7 common/src/main/java/me/lucko/luckperms/common/calculator/PermissionCalculatorBase.java resolves permissions via ordered processors with cache invalidation; an arbitrary partial graph must not be treated as a permission oracle. Paper/Purpur Brigadier informs future parity; importing the Java dispatcher is not justified to fix metadata. FerrumC command details need further checking during wire implementation; no verified pinned direct example is claimed for this particular bug. Folia scheduling and Horizon login packets do not change command identity parsing, EssentialsX utilities rely on consistent command effect names, and e4mc/Krypton/PulseNet packet networking is inapplicable.

Alternatives: retain mixed-case stored names and normalize at each dispatch (error-prone), reject uppercase (breaking previously accepted registrations), or selected normalize Command.name/aliases at registration before publication. For legacy Grants: arbitrarily increase max graph depth (same latent bug), or selected keep bounded walk and deny on saturation if any additional distinct graph nodes are reachable. Preserve default group and in-range cycles. No claims of LuckPerms parity.

Acceptance: demonstrate failing-before uppercase command and graph overflow tests, fix both, verify normal/default group, graph within boundary, alias effect, registry version/projection; full Rust suite, release, integration fixtures, review gate. First-party AGPL only; no upstream code transplanted, no unmeasured performance claims.


### RV-CMD-01 additional registration validation before patch

Further local inspection found that a Command parameter of Argument::Integer with min greater than max is accepted into the registry even though Registry::execute rejects every value with its inclusive range check. SteelMC command/protocol.rs only serializes valid argument types from its Brigadier typed dispatcher; our stand-alone Command registry should reject impossible argument metadata transactionally before publishing a protocol-neutral tree. Plan: add failing-before test of min=10, max=1 that rejects registration and confirms tree/revision unchanged, then validate min <= max before inserting routes. Do not change parser behavior or claim full Brigadier parser parity.


## RV-CMD-01 post-implementation review: canonical dispatch, bounded grants and argument metadata (2026-10-23)

Verified starting state: clean branch 9c04118c89d707e5e1e76a85f74d689ee9283019, 65/65 Rust tests. Three new regressions failed against original code BEFORE changes:
1. mixed_case_registration_has_one_canonical_identity_everywhere: registered Demo:Run, but alias RuN returned Effect::Core command Demo:Run, not normalized demo:run.
2. legacy_grants_reject_incomplete_group_inheritance: a direct Allow remained true when the legacy command.rs Grants helper truncated its >=64 subject BFS without finishing the permission graph.
3. invalid_integer_range_is_rejected_without_mutating_registry: invalid Argument::Integer min 10 max 1 was accepted even though no execution could satisfy the range. The first version of this regression used an invalid apostrophe-containing alias, which was rejected for an unrelated reason; it was corrected to a valid alias and then failed against the actual numeric bound omission.

Implementation in src/command.rs: normalize stored Command.name and Command.aliases to the validated lowercase routes in Registry::register before publishing. Reject min > max integer argument metadata before any route insertion. In the lightweight Grants helper, traverse all reachable named groups with deduplication; return false if adding a distinct subject would exceed 64, including optional default group, instead of deciding from a partial grant set. Preserve accepted normal group and cycle behavior. An additional nested fanout/cycle regression covers 64 subjects and 65 subjects. This helper is a test/extension compatibility helper, NOT the always-on NativePermissions database (whose graph limit is independently 128).

Four added Rust regression tests brought the suite to 69/69 passing, rustfmt and release build passed. No claim about runtime throughput or JVM/Paper/SteelMC/Brigadier wire parity. SteelMC normalization/validation and visible packet projection, and LuckPerms processor/cache source findings were compared before the patch; this implementation remains a smaller protocol-neutral native command registry with no player command packet or suggestions negotiation.

Remaining REV-CMD work: actual Mojang Brigadier grammar and vanilla argument parser IDs; command graph wire serialization for Minecraft protocol 777; auth-aware player command sources; alias collision testing at 1024 commands; permission resolution differential oracle; completion range accuracy, authorization edge cases, plugin lifecycle ordering and live-client parity. Do not approve REV-CMD until these have direct upstream source comparison and tests. No upstream code copied, code AGPL-3.0-or-later.


## RV-UTIL-01 pre-implementation review: legacy JSON home/warp key case collision (2026-10-23)

Current Rust NativeUtilities normalizes user/home/warp keys to ASCII lowercase on all administrative writes. However open() loads old or externally generated JSON v1 keys and validates them without the same normalization. An imported user "Alex", home "Base", or warp "Spawn" passes the schema but home("alex","base") or warp("spawn") returns None. A subsequent write can introduce both Alex and alex keys into the persisted store. A collision may contain different locations: merging silently loses user data.

Pinned sources inspected: EssentialsX e70bdb817c758474acb3a23a37cf961b2f716a50 Essentials/src/main/java/com/earth2me/essentials/UserData.java constructs user YAML paths using base.getUniqueId and treats the last username as metadata; setHome uses StringUtil.safeString for stored home names, and Commandhome.java lowercases a requested name before dispatch. EssentialsX does not establish that our ASCII-casefold map is exact parity; its UUID-backed user storage is the correct future integration target. Paper/Purpur provides Bukkit world/location and permission/event contracts for actual teleport; Folia requires correct entity/region-owned scheduling for cross-world teleports, neither implemented in this JSON-only module. SteelMC/FerrumC persistence approaches were already reviewed in REV-STORAGE; no database migration is warranted to normalize a v1 key. LuckPerms permission name canonicalization is analogous but does not define EssentialsX home/warp naming; Horizon client status/packets and specialist network tools do not apply.

Alternatives: leave imported mixed-case values inaccessible (inconsistent), silently merge colliding keys (data loss), reject all uppercase legacy values (needless incompatibility), or CHOSEN normalize legacy v1 keys on load only when unambiguous, rejecting collisions in user names, per-user home names, or warp names with InvalidData. Preserve Position.world case (world identifiers have independent semantics), location coordinates, and JSON v1 schema. A new successful mutation writes the normalized data, without speculative rewrites at open. This matches the existing Rust write/read contract, not unverified EssentialsX file compatibility.

Acceptance: first reproduce failing mixed-case JSON lookup and collisions; after fix, verify imported mixed-case user/home/warp are read and rewritten on next successful store mutation, but any ambiguous collisions fail closed, including separate case variants of warp and home names. Full 69-test Rust baseline, release and plugin/storage smoke, audit review hold, no performance claim or upstream source copied.


## RV-UTIL-01 post-implementation verification: imported JSON v1 case-collision safety (2026-10-23)

Actual failing-before tests: unambiguous_legacy_case_keys_load_and_save_canonically panicked because home("alex","base") returned None for persisted user Alex/home Base; imported_case_collisions_fail_closed_in_all_key_spaces panicked because the loader accepted duplicate logical user, home, and warp keys with different capitalization. Both fail despite the previous writer already lowercasing newly inserted entries.

Implementation: src/builtin/utilities.rs::NativeUtilities::open now normalizes preexisting JSON v1 user, home and warp keys using the same ASCII-casefolded name validation as all new writes. It refuses any casefold collision in user IDs, nested home names, or warp names with InvalidData rather than merging independent coordinate records. World IDs and all coordinates are preserved, including exact original world spelling; persisted schema remains v1. Data is canonicalized in memory on open and written in canonical form on the NEXT successful mutation, not eagerly rewritten. The service remains a console-only data store; it does not authenticate users, validate world existence, perform teleport or match full EssentialsX home limits/events.

Evidence: two new Rust regression tests cover successful unambiguous imports/round trip and all three collision cases, bringing full Rust 1.90 suite from 69 to 71 passing. Release build succeeded. scripts/test_single_writer.py was extended with a separate temporary data directory to start an actual Rust server using imported v1 upper-case keys, query both a home and warp, write a new warp, assert persisted casefolded JSON and reject startup with a deliberately introduced case-colliding warp. Existing two-writer, clean/SIGKILL restart checks continue to pass. No real player identity or Paper/Folia world ownership validation was performed; no performance gain or exact EssentialsX parity claimed.

Decision: keep v1 JSON compatible where unambiguous and fail startup on contradictory aliases instead of discarding or silently choosing one. Required next: UUID-backed authenticated principals, rename/index migration strategy across mixed case and UUIDs, complete EssentialsX world/home/warp semantics, and async owner-affine world teleport. RV-UTIL moves from PENDING to IN_REVIEW; REV-STORAGE stays IN_REVIEW since database durability/scaling remains unapproved. Original AGPL code, no third-party source copied.


## RV-TICK-01 — pre-implementation review: console work time fairness (2026-10-23)

**Observed local behavior:** src/main.rs::main has a 20Hz (50ms target) monotonic Instant clock and an independent TCP accept thread. The console receiver is a bounded 1,024-element sync_channel, drained with rx.try_iter().take(128) each tick; a 128-command count bound does not bound time consumed by expensive native disk mutations or plugin callbacks, so a burst can monopolize the simulation tick. The server does not yet run actual worlds, so 20TPS remains a target, not demonstrated gameplay TPS.

**Pinned and relevant source/code comparisons:** SteelMC dc8eaba steel-core/src/command/queue/requests.rs defines 1,024-capacity execution/suggestion queues, coalesces suggestions and limits command requests to 128 per tick, separating request sources; this is a valid count-budget precedent but does not guarantee a bounded wall-clock time for callbacks. PaperMC/Paper paper-server/patches/sources/net/minecraft/server/MinecraftServer.java.patch records tickTimes* and MSPT/TPS timing, uses a tick schedule and an explicit main-thread processQueue, illustrating the need to distinguish target TPS from actual execution and tick latency. PaperMC Folia official design https://docs.papermc.io/folia/reference/overview/ establishes independent 50ms region deadlines and immediate scheduling for overdue ticks without claiming independent-region behavior here; actual Folia region-owner services are not implemented in our standalone server. Horizon's client/engine and FerrumC world work are future architecture references but do not define the console-only time budget; LuckPerms/EssentialsX are potential synchronous command callbacks, not authoritative tick schedules; e4mc, Krypton and PulseNet network batching are unrelated.

**Alternatives:** A keep count-only 128 cap (may spend arbitrary tick time), B send all callbacks off-thread (unsafe for command ordering/native stores and eventual world ownership), C add a 10ms cooperative wall-clock cutoff on top of the existing 128-command cap before dequeuing each additional command, leaving remaining FIFO messages queued (CHOSEN). Time budget is a conservative prototype policy, not a vanilla/Paper setting; 10ms leaves nominal headroom for other tick work. Preserve 1,024 bounded ingress and stop handling, do not flush or drop a dequeued command on reaching cutoff. A single blocking callback can still exceed the time cap, so this is mitigation, not strict real-time enforcement. If execution queues later split by actor/region, account for Folia owner-affine execution and benchmark under load before refining.

**Acceptance:** extract the existing count-bound draining into a small tested helper without changing existing behavior; write a failing-before regression showing that a first callback taking >10ms still lets a queued second command run in the same tick; then enforce pre-dequeue elapsed cutoff. Include direct 128-command count boundary, FIFO preservation, Stop semantics and empty queue; run 71 existing Rust tests, release and server/JVM smoke, source-ledger SHA and expected Phase -1 enforcement failure. No external source copied; no throughput/real-TPS claim without a controlled benchmark.


## RV-TICK-01 — post-implementation verification: cooperative console time cutoff (2026-10-23)

Initial state at commit 3ad9e642eef9ab0509714a9a84973a4bb1942de9: clean horizon-rust worktree, 71/71 Rust tests. The existing console command processor drained up to 128 commands per tick with no elapsed-time check. A behavior-preserving extraction into drain_console_commands and a 17ms synthetic slow handler test reproduced the defect: old processing ran all three queued messages in one tick, so the test asserting only the first should run FAILED. An independent Stop test confirmed shutdown leaves subsequent queued messages in FIFO.

Fix: src/main.rs::drain_console_commands accepts a cooperative Duration budget. The actual main loop uses MAX_CONSOLE_TIME_PER_TICK=10ms (20% of the 50ms target tick); a configured 128-command cap still applies. Before removing any subsequent command from the queue, the drain checks elapsed monotonic time and stops if it has met the budget, leaving pending FIFO entries intact. The first available command is allowed to execute even with zero budget, ensuring forward progress; a malicious/blocking callback can still run arbitrarily long and stall the tick. Stop returns early without consuming subsequent messages. The network accept thread remains independent, and tick deadline/resynchronization policy is unchanged. A 10ms budget is an early local policy, NOT an upstream SteelMC/Paper/Folia magic value.

Verification: three new Rust tests cover a slow 17ms callback preserving two queued commands, early Stop preserving later commands, and a zero-length budget allowing one message then retaining FIFO order. The existing bounded 128-command and 1,024-capacity tests remain. 74/74 Rust 1.90.0 tests and release compilation passed after implementation, and full real-server/script lifecycle checks ran. The elapsed-time rule was tested under a synthetic delayed callback; no real player/world tick, TPS, p95 MSPT, plugin isolation, preemption or total command throughput was established. This logic would need redesign with owner-affine Folia region tasks and typed player requests. No third-party code copied; first-party AGPL-3.0-or-later.

RV-TICK remains IN_REVIEW. Outstanding: actual 20Hz simulation work/metrics, 5s/1m measured TPS/MSPT history, tick overrun policy parity, region ownership and cross-thread handoff validation, plugin callback isolation, integration time-load profiling, and player queue fairness. Compare those against pinned Paper/Folia/SteelMC before implementing. The Phase -1 review gate remains ACTIVE.
