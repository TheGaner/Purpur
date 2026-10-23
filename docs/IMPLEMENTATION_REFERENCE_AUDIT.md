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
