# Purpur parity tracker

| Area | Status |
| --- | --- |
| Standalone Rust binary | Prototype |
| Status / ping / bounded VarInt protocol 777 | Implemented with tests |
| Playable login / encryption / auth | Not implemented |
| Horizon-only client authentication | Not implemented |
| World simulation / persistence / generation | Not implemented |
| Blocks, entities, combat, inventory | Not implemented |
| Rust console commands, aliases, parameters, permission checks | Foundation with tests; no player command packet support |
| Selectors, persisted permissions, protocol commands | Not implemented |
| Purpur configuration and behavior parity | Not implemented |
| Rust plugin ABI v1 and example | Prototype |
| Rust WASM sandbox | Not implemented |
| Java jar discovery and lifecycle | Experimental, no Bukkit API |
| Bukkit, Paper, Purpur Java plugin compatibility | Not implemented |
| Tick scheduler event integration | Prototype, no world |
| Full parity suite against Purpur 26.3 | Not implemented |

Every feature must pass deterministic reference tests before marking parity complete.

For release gates and per-upstream integrations see docs/ROADMAP.md.

Network milestone (2026-10-23): dedicated acceptor and state-bounded status/login parsing are implemented and locally benchmarked. 48 Rust tests passed. Login authentication, Horizon-only identity proof, configuration, play, TPS and worldgen remain unimplemented/unverified.
