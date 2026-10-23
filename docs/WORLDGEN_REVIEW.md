# SteelMC / FerrumC review

SteelMC has a steel-worldgen crate with biomes, density, noise, surface, structures and generated vanilla registry functions. It depends on steel-registry, steel-utils, steel-math, generated assets and nightly portable_simd. It cannot be dropped into this small stable Rust server without integration work. Worldgen source is **not imported yet**.

SteelMC reports 7,500 reference chunk comparisons (2,500 per dimension), and a 10,201 Overworld chunk benchmark on a Ryzen 9950X. These are upstream claims for the verified version and hardware, not this project's results. SteelMC remains pre-alpha; other game systems are incomplete.

FerrumC (MIT) currently describes a new rework/v2-skeleton branch, and its published master README targets different Minecraft versions. Focus on NBT, storage and queue designs as comparisons; do not assume worldgen parity transfers across protocol/game versions.

Proposed generator: deterministic WorldgenProvider with immutable chunk results, a bounded work graph, per-worker temporary arenas and region-thread commit. Benchmark cached density subexpressions, fast-batched noise and climate sampling, structure-stage queues and coalesced requests.

Testing policy: preserve a small representative/golden set including seams and RNG seeds, run affected-domain tests when optimizing specific functions, and reserve the full upstream suite for large periodic/version change gates. Optimizations that alter noise/biome/structure outputs require stronger targeted comparisons even if baseline was previously validated.

SteelMC worldgen reference pin (not copied): dc8eaba7ff21c348226a05741c2fb9809a8ca90d. Full dependency and code-header audit still required before an import.
