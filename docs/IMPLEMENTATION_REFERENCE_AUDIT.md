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
