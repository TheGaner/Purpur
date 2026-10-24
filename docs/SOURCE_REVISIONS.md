# Source revisions for future integration

These are read-only upstream HEAD snapshots captured on October 23, 2026. They are reference pins, **not copied code**. Update a pin, check the license and generate new parity results before importing. No Git dependency currently points to these revisions.


## How source pins are used

**Purpur 26.3 is the PRIMARY server behavior reference.**
Inspect the pinned assembled Purpur server semantics (Purpur patches
PLUS inherited Paper/vanilla implementation), then compare candidate
Rust architectures from SteelMC/FerrumC, Horizon protocol and component
model ideas, Folia scheduling, and optional LuckPerms/EssentialsX service
semantics. The winning design may be structurally different from Java
but must reproduce chosen observable outputs unless a divergence is
explicitly documented and approved.

The below commits are immutable reproducibility anchors, not proof of
full Purpur parity, current upstream HEAD, or permission to copy code.
The source-audit fidelity matrix and required behavioral differential
fixture contract are in IMPLEMENTATION_REFERENCE_AUDIT.md.

Concrete Purpur entry points read at the pinned SHA:
- https://github.com/PurpurMC/Purpur/blob/5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0/purpur-server/src/main/java/org/purpurmc/purpur/PurpurConfig.java
  global YAML/defaults and command registration
- https://github.com/PurpurMC/Purpur/blob/5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0/purpur-server/src/main/java/org/purpurmc/purpur/PurpurWorldConfig.java
  per-world fallback and configuration migration



Purpur 26.3 executable reference artifact (independently verified):
- Official build ID: 2654; API: https://api.purpurmc.org/v2/purpur/26.3/2654
- Source commit in build metadata: 5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0, matches pinned source
- Download URL: https://api.purpurmc.org/v2/purpur/26.3/2654/download
- Exact size: 58,123,798 bytes; upstream MD5: 5cd522f99592a3545913a30e44665de1
- Locally measured SHA-256: a22d86bb993491a2d4fe77eca77c247541f1720ce19f68c7af0881c443b9878e
- ZIP integrity: passed, 580 entries; artifact downloaded into disposable directory, then deleted
- **No server execution or Minecraft EULA assent occurred**. This is build
  provenance, NOT a Purpur behavior oracle capture; see RV-STATUS-03.

| Reference | Upstream ref | SHA | License |
| --- | --- | --- | --- |
| [SteelMC](https://github.com/Steel-Foundation/SteelMC/tree/dc8eaba7ff21c348226a05741c2fb9809a8ca90d) | master | dc8eaba7ff21c348226a05741c2fb9809a8ca90d | AGPL-3.0-or-later |
| [FerrumC](https://github.com/ferrumc-rs/ferrumc/tree/51830e99675deae1f6cdbb62bf5d32b3d25c1c76) | master | 51830e99675deae1f6cdbb62bf5d32b3d25c1c76 | MIT |
| [LuckPerms](https://github.com/LuckPerms/LuckPerms/tree/b93a4d7b24fbcb97170e355d651984493ab64e79) | master | b93a4d7b24fbcb97170e355d651984493ab64e79 | MIT |
| [EssentialsX](https://github.com/EssentialsX/Essentials/tree/e70bdb817c758474acb3a23a37cf961b2f716a50) | 2.x | e70bdb817c758474acb3a23a37cf961b2f716a50 | GPLv3 |
| [Folia](https://github.com/PaperMC/Folia/tree/8cffa9059ad6bbc2b28510c38ed61a51098d4971) | ver/26.3.x | 8cffa9059ad6bbc2b28510c38ed61a51098d4971 | GPLv3 patches |
| [Purpur](https://github.com/PurpurMC/Purpur/tree/5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0) | ver/26.3 | 5cd359666a40bc4a41ce3aa7c755d2ad015ebfb0 | MIT |
| [Horizon](https://github.com/ludovic111/horizon/tree/e325fa65878071d5c9076e908e9a4adb258b3313) | main | e325fa65878071d5c9076e908e9a4adb258b3313 | MIT |
