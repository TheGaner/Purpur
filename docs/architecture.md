# Required architecture

- Rust standalone executable and simulation (no Horizon engine process, Java server, or original Purpur server).
- Horizon-only client target. Protocol 777 is *also vanilla*; authenticating Horizon needs a new client-server capability handshake. Horizon currently sends standard minecraft:brand on configuration. Do not enforce this until Horizon client supports it.
- Reference versions: Horizon https://github.com/ludovic111/horizon and Purpur https://github.com/PurpurMC/Purpur (ver/26.3).
- Horizon's dedicated-server supervisor launches horizon-engine/horizon-client --dedicated. Do not use this for our standalone runtime.
- Horizon mods are Wasmtime component-model WASM plugins with WIT imports, capabilities, memory/time budgets. Server-side mod hook is not yet active. Port the host API concepts, not the client runtime.
- Stable plugin events should support cancellable events, execution priority, commands, scheduler, world API, permissions and thread affinity.
- Rust plugin path: current unsafe native cdylib ABI is temporary; move to Rust SDK compiled to WASM with Wasmtime isolation.
- Java plugin path: JVM plugin service, Bukkit/Paper/Purpur facade classes and bridge event RPC. Current worker only demonstrates JAR discovery and enable/disable when classes can load without Bukkit symbols.
- Java code and native Rust plugins can execute arbitrary code. Never claim sandboxing.

## Development milestones
1. Complete protocol 777 login, encryption/authentication, configuration and play; prove real Horizon connection.
2. World state: blocks, chunks, NBT, storage, worldgen, tick scheduler, entities, collision, movement, inventory.
3. Vanilla commands (and 26.3 command parity tests), scoreboard, selectors, permissions, advancements, gameplay.
4. Port Purpur configs and behavior option by option with traced parity tests.
5. Versioned server Rust WASM SDK plus event and command dispatcher, safe host capabilities.
6. JVM bridge API, plugin.yml and paper-plugin.yml dependency graphs, event/command dispatch, Bukkit/Paper compatibility tests.
7. Horizon client opt-in authentication/identity handshake; strict reject non-Horizon client.

Command layer uses the new standalone src/command.rs registry; source license is AGPL-3.0-or-later. See docs/ROADMAP.md, docs/LICENSING.md and THIRD_PARTY_NOTICES.md.

## Architecture research decision, 2026-10-23

The Rust command graph is deterministic and source-filtered, but intentionally remains independent of network serialization while no player login/play runtime exists. The future command path uses versioned immutable graph snapshots and permission/context revisions, with bounded tick queues. See docs/REFERENCE_ARCHITECTURE_RESEARCH_2026-10-23.md.
