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

The optional trusted Rust ABI export horizon_plugin_commands returns a static
UTF-8 C string with lines in this shape:

    hello|example.hello|hi

A line declares command name, permission node, comma-delimited aliases.
Plugin execution uses the existing event callback (event 3). No world/player
commands or runtime Java plugin command registration are implemented yet.
Player permissions are fail-closed and an in-memory contextual/group permission
provider is scaffolded; no players may connect yet and persistence/LuckPerms
integration are planned.

See docs/ROADMAP.md, docs/LICENSING.md and THIRD_PARTY_NOTICES.md.
