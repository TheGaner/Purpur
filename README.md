# Purpur-Horizon (Rust) — experimental, independent server

This branch of the Purpur fork contains a NEW Rust server intended only for the Horizon client, and targets Purpur behavior parity. It runs without starting the Horizon client or engine.

**Current stage:** Status/ping on Minecraft 26.3 protocol 777, explicit login rejection because the gameplay engine is not yet implemented, 20 Hz event prototype, Rust native plugin ABI v1, and an optional JVM Java JAR lifecycle bridge. This is NOT a playable Minecraft server or a Bukkit/Paper/Purpur plugin-compatible server yet.

## Build

    cargo test
    cargo run -- --bind 127.0.0.1:25565 --motd "Rust server"

On this development PC, Rust 1.90 was installed under the project-local
.rustup folder so the host's default Rust configuration stays unchanged:

    RUSTUP_HOME="$PWD/.rustup" cargo +1.90.0 test
    RUSTUP_HOME="$PWD/.rustup" cargo +1.90.0 run -- --bind 127.0.0.1:25565

A fresh clone on a different machine needs a Rust 1.90+ toolchain installed
through rustup before using the normal cargo commands.

Commands: stop, version, plugins, help.

## Native Rust plugins (trusted only)

    cargo build --release --manifest-path examples/rust-plugin/Cargo.toml
    mkdir -p plugins/rust
    cp examples/rust-plugin/target/release/libhello_horizon.so plugins/rust/

Rust native plugins implement exported ABI v1 and event callback; these libraries have arbitrary OS privileges. Wasmtime-based isolated Rust-WASM plugins are planned and are more appropriate for untrusted code.

## Java plugins

Place JAR files with plugin.yml in plugins/java. Java 21+ is needed only if JARs are installed. A separate Java process reads plugin.yml and invokes no-argument onEnable/onDisable methods. This **does not** implement Bukkit/Paper APIs, so ordinary existing Purpur plugins WILL NOT YET LOAD. The full bridge needs API emulation and bidirectional callbacks.

See docs/architecture.md and docs/parity.md.


## Standalone distribution

A compiled purpur-horizon binary can be copied and run without the Horizon
source tree, the Horizon engine, or the Purpur Java server. The Java bridge
source is embedded in the Rust binary and extracted only when Java jars are
present. A system Java 21+ runtime is still required for Java plugins.
This does not change the existing Java API compatibility limitation.

## Command layer (early implementation)

Native Rust command registry now supports quoted and escaped token parsing,
aliases, plugin namespaces, collisions, positional word/integer/greedy arguments,
metadata-based registration, and permission-scoped discovery, execution and
suggestions. The console dispatches commands through this registry instead of
a hardcoded command switch. Current built-ins are help, stop, version, plugins,
uptime, tps (target only) and echo. Try: help, help echo, echo "hello world",
hi (from the compiled example plugin), and stop.

For native command declarations, use the optional manifest ABI v2:
horizon_plugin_commands_v2(out: *mut u8, capacity: usize) -> usize.
The native plugin writes at most capacity bytes into the caller-owned
buffer and returns the actual byte length. The server caps it at 8192 bytes,
validates UTF-8 and rejects NUL bytes. No null terminator is needed.
The payload remains newline-separated, with lines in this shape:

    hello|example.hello|hi

Each line declares command name, permission node and comma-delimited aliases.
The old pointer-returning horizon_plugin_commands v1 symbol is deliberately
ignored to eliminate a metadata read with no bound; native plugins using it
can still load and receive lifecycle events, but must be rebuilt to register
commands. Native dylibs have full process privileges and are not sandboxed.
Plugin execution uses the existing event callback (event 3). No world/player
commands or runtime Java plugin command registration are implemented yet.
Player permissions are fail-closed and an in-memory contextual/group permission
provider is scaffolded; no players may connect yet and persistence/LuckPerms
integration are planned.

See docs/ROADMAP.md, docs/LICENSING.md and THIRD_PARTY_NOTICES.md.

## Source comparison and optimized architecture

See docs/REFERENCE_ARCHITECTURE_RESEARCH_2026-10-23.md for detailed architecture evidence and roadmap decisions. The new command-tree projection is protocol-neutral, source/permission-filtered and versioned; it is not yet transmitted to a client. Console ingress is capped at 1,024 pending commands with at most 128 executed per tick, providing explicit resource bounds rather than a measured throughput claim.

A live console diagnostic command, commandtree, reports the permission-filtered
registry projection's node count, revision and validity. This is explicitly
not the Minecraft command-tree wire packet or client gameplay support.


## Always-on native Rust services

Native permission/group and home/warp data services now load by default, with no Java or plugin installation. Use --data DIR to locate persistence (default data/, ignored by Git). Invalid JSON fails startup rather than silently discarding rules or locations.

Console examples: nativefeatures ; permissions set alex essentials.home allow ; permissions check alex essentials.home ; permissions parent alex staff ; sethome alex base minecraft:overworld 1 64 2 ; homes alex ; setwarp town minecraft:overworld 2 64 3 ; warp town.

Homes and warps currently store and report coordinates; teleport is blocked until player/world simulation is implemented. The native modules are independently written early subsets, NOT complete official EssentialsX or LuckPerms ports. See docs/ROADMAP.md and docs/LICENSING.md for source combination gates.

## Network admission and measured status performance

Network acceptance is independent of the 20Hz game loop. Active handshakes are bounded by --max-connections N (default 64), and each preauth packet family is limited before payload allocation. The protocol 777 login hello is parsed, but the server still explicitly rejects all player admission: there is no authentication or playable world.

Run python3 scripts/bench_status.py target/release/purpur-horizon to measure localhost server-list status requests. Sequential median RTT improved from roughly 50ms to roughly 0.12ms in the recorded test, not a TPS benchmark. See docs/NETWORK_OPTIMIZATION_2026-10-23.md for methods, p95/p99, restrictions and remaining priorities.

## Retrospective security review (ongoing)

The optional Java bridge is shut down by closing stdin, waiting up to 1.5
seconds for clean disable, and terminating the child if it hangs. Preauth
protocol reads share a three-second monotonic deadline so a slow client
cannot indefinitely reset socket timeouts one byte at a time.

To run the full native/JVM regression, after building release server and
example plugin, run python3 scripts/test_plugin_lifecycle.py. The complete
cross-server retrospective audit remains open: see
docs/IMPLEMENTATION_REFERENCE_AUDIT.md and docs/ROADMAP.md Phase -1.
No full Bukkit compatibility, Horizon-only authentication or gameplay yet.

## Persistent native data: single-writer policy

The server still uses native JSON v1 data files. Each permissions/utilities
database holds an exclusive advisory lock on its own .json.lock sidecar while
open. Running two server instances against the same data directory fails
closed, rather than silently overwriting grant or home/warp snapshots.
Lockfile paths remain on disk after shutdown; locks are released by the OS.

Each save checks the 2 MiB limit before writing a unique create_new temporary
file, syncs it, renames it, and on Unix syncs the containing directory. This
improves integrity but is not a substitute for SQLite/LMDB transactions,
full fault-injection/power-loss validation or migrations.

Build a release binary and run python3 scripts/test_single_writer.py to
verify real-process exclusion and restart persistence.


### Ambiguous snapshot-write failure handling

An error after atomic rename, such as a directory-sync failure, may have
already changed the on-disk snapshot. Both native stores now mark themselves
unusable for further mutations until restarted/reopened, rather than risk
overwriting the new disk data with stale state. Permission checks return
Undefined (deny) and home/warp lookups hide stale locations while the store
is in this state. Errors before file replacement remain retryable.

This was tested with injected errors on the real JSON writer, and separately
with a clean and SIGKILL server restart. It does not prove resilience to a
real power cut during an unsafe filesystem or across distributed stores.


### Legacy permission subject normalization

Name-based native permission subjects and group links are now ASCII-casefolded
on administrative writes and lookups. Existing version 1 JSON permission files
with unambiguous legacy capitalization load and normalize in memory; their
normalized representation is saved on the next successful write. An old file
containing two case variants of the same subject fails startup rather than
combining potentially conflicting security grants. Administrators must resolve
those duplicate records manually. This is a pre-playable console convenience,
not an authenticated player identity system: future player permissions must be
keyed by validated UUIDs, not client-supplied names or UUIDs.


### Native permission ancestry safety

The current permission evaluator has a defensive maximum of 128 distinct
subjects in a single user/group ancestry walk. If the limit is exceeded it
returns Undefined (denied), never an Allow calculated from an incomplete
graph. Persisted v1 JSON may contain only nonempty user: or group: subject
keys, and parent links must target a nonempty group: identity. Malformed
inheritance and identity namespaces fail startup instead of being followed.
These are safety boundaries, not LuckPerms-compatible context, weight or
default-group semantics. Large legitimate inheritance graphs may be denied
until the cap is redesigned and tested against a reference oracle.


### Typed console permission identifiers

Built-in permission console commands accept a legacy user as alex or
user:alex; both refer to the same stored user subject, case-insensitively.
Group rules can be edited using group:staff, and parent/unparent commands
accept a group argument of staff or group:staff. A user permission check
will never reinterpret group:staff as a user with group authority; malformed
or doubly qualified names are rejected. These inputs are administrative
labels only, not authenticated identities for online players. Actual
Horizon login/authenticated UUIDs and LuckPerms parity remain pending.


### Optional status Ping timeout

After serving a server-list status response, the server waits at most one
additional second (and never past the original three-second handshake
deadline) for an optional protocol-777 Ping packet. The reduced idle worker
hold has a localhost regression, including a delayed Ping/Pong round trip.
Clients taking more than one second to send Ping after receiving status
could miss their Pong. This is a prototype admission tradeoff, not a
production network-concurrency or Minecraft gameplay benchmark.


### Reviewed command registration safety (RV-CMD-01)

The Rust command registry canonicalizes stored command names and aliases to
lowercase, so registrations, permission-filtered trees and dispatcher effects
agree even when an extension registers a mixed-case name. Inverted integer
bounds are rejected before publishing a command. The separate legacy
in-memory Grants test helper now denies when group traversal exceeds its
64-subject bound rather than granting from a partial graph. The production
native permission evaluator has different data, semantics and limits. This
does not provide full Brigadier or Minecraft protocol-777 command support.


### Legacy home/warp JSON compatibility

Native utility data still uses JSON version 1 and ASCII-casefolded
administrative home/warp names. On startup, existing imported JSON files
with uppercase names are normalized in memory if unambiguous. Conflicting
case aliases for user IDs, per-user homes or warps cause a fail-closed
InvalidData startup error; records are NOT silently merged or discarded.
Canonical names are persisted after the next successful mutation.
World identifiers and coordinates remain unchanged. This is not a real
teleport service or authenticated UUID ownership implementation.


### Cooperative console command time budget

The simulation tick targets 20 Hz, but gameplay simulation and actual TPS
reporting are not implemented. Console command processing is bounded by
1,024 waiting messages, at most 128 handled per tick, and a new cooperative
10ms wall-clock time budget: after a handler returns, the loop checks the
deadline before dequeuing more commands. This preserves queued commands
instead of silently dropping them, including after an expensive action.
At least one pending command may make progress each tick; a single blocking
native/plugin callback can still take longer than 10ms. This is not strict
real-time isolation or complete Folia/Paper scheduler behavior.


### Dependency provenance check

Rust dependencies are pinned by Cargo.lock. Run
python3 scripts/check_cargo_licenses.py (using the repository-local Rust
1.90 toolchain) to verify all registry crates against the reviewed
docs/CARGO_DEPENDENCY_LICENSES.json license declaration snapshot. The
checker reports any new/changed license or checksum for manual review.
This does **not** certify per-file license compliance or generate full
redistributable third-party license text bundles. The release-level
copyright/notice and AGPL matching-source checklist remains open.


### Outbound admission deadline

Pre-play network responses (server-list status, Ping/Pong, login rejection)
now share the original three-second monotonic handshake admission deadline.
The Pong also uses the shorter one-second optional Ping grace. Reads and
writes recompute their remaining socket timeouts for partial I/O so one
stalled send cannot restart a fresh three-second window. Real-TCP tests
exercise expired, successful framed and blocked-receiver writes. These
limits do not constitute authentication, a per-IP admission quota, or
guaranteed real-time process cancellation.


### Review-source drift detection

The mandatory Phase -1 coverage check now validates current SHA256 content
digests for all Rust, Java and Python source files owned by in-review or
approved subsystems. It rejects a changed file or missing hash until an
updated reviewed digest and evidence are recorded in the existing ledger.
A pending review can remain pending without digests. This is not an
approval mechanism: --enforce still requires full upstream comparison,
testing and sign-off for every subsystem.


### Preauthentication connection fairness

The independent TCP acceptor now enforces both a global preauthentication
worker limit (default 64, --max-connections N) and a configurable simultaneous
per-source-IP limit (default 8, --max-per-ip N). Both accept values 1..1024;
the overall limit always applies. Permits are released automatically on
worker return, errors, or panic; IPv4-mapped IPv6 and IPv4 peers share one
source allowance. The peer address is the actual TCP peer, never the
untrusted Minecraft handshake hostname. Run
python3 scripts/test_peer_admission.py after building the release binary to
exercise rejection, other-source admission, expiry/release and invalid CLI
settings using Linux loopback addresses.

The per-IP cap is *simultaneous occupancy only*, not an attempt-rate limiter
or IP authentication. Users behind NAT or a reverse proxy may share one
quota; operators should tune the option accordingly. This prototype still
does not accept authenticated gameplay logins.


### Local preauth fairness regression and benchmark

After cargo +1.90.0 build --release, run python3 -B
scripts/test_peer_admission.py for the fast correctness fixture and
python3 -B scripts/bench_admission.py for a separate controlled saturation
experiment. The latter holds eight 127.0.0.1 connections at a per-IP quota
of eight, validates other-source status response, same-source refusal and
recovery, and reports status RTT/refusal p50/p95/p99 and sample counts.
Uses Linux 127/8 loopback, not a remote traffic/load generator. Results
are not gameplay TPS, production throughput or a DDoS protection claim.


### Native permission lookup limits

The native JSON permission store and its in-process resolver now apply
consistent input limits: permission node and world identifier names must
be valid ASCII identifiers of at most 128 bytes (with the existing
wildcard-node syntax where applicable). Malformed, empty or oversized
lookup strings return Undefined before entering the 8,192-entry cache.
This prevents oversized lookup keys from retaining unbounded memory, but
does not establish authenticated remote access or LuckPerms parity.


### Native permission cache and ancestry bounds

Inherited permission groups are now marked visited when first queued,
so a convergent graph cannot repeatedly enqueue the same subject.
More than 128 distinct reachable subjects continues to return Undefined
rather than making a decision from a partial graph. An unrelated
expiring node/world permission no longer prematurely invalidates the
cached answer for another requested node/world. Relevant expiring
rules still invalidate the decision at the expiration timestamp.
These guarantees do not establish full LuckPerms priority/context parity
or a measured performance improvement.
