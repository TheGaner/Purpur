# Purpur-Horizon (Rust) — experimental, independent server

This branch of the Purpur fork contains a NEW Rust server intended only for the Horizon client, and targets Purpur behavior parity. It runs without starting the Horizon client or engine.

**Current stage:** Status/ping on Minecraft 26.3 protocol 777, explicit login rejection because the gameplay engine is not yet implemented, 20 Hz event prototype, Rust native plugin ABI v1, and an optional JVM Java JAR lifecycle bridge. This is NOT a playable Minecraft server or a Bukkit/Paper/Purpur plugin-compatible server yet.

## Build

    cargo test
    cargo run -- --bind 127.0.0.1:25565 --motd "Rust server"

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
