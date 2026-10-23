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
