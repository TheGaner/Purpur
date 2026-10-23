# Third-party attribution and provenance

No implementation files from the named upstream projects are currently copied into this Rust-only server branch. This repository is a fork of the Purpur GitHub repository, but horizon-rust begins as an independent orphan Rust tree; original Java Purpur remains on ver/26.3.

| Upstream | License observed | Code imported |
| --- | --- | --- |
| https://github.com/PurpurMC/Purpur | MIT | None |
| https://github.com/ludovic111/horizon | MIT | None |
| https://github.com/Steel-Foundation/SteelMC | AGPL-3.0-or-later | None (worldgen planned) |
| https://github.com/LuckPerms/LuckPerms | MIT | None |
| https://github.com/ferrumc-rs/ferrumc | MIT | None |
| https://github.com/EssentialsX/Essentials | GPL v3 | None (optional Java integration planned) |
| https://github.com/PaperMC/Folia | GPL v3 patches and inherited licenses | None (region scheduler design only) |

Future source imports MUST add an immutable upstream commit SHA, exact imported paths and modifications, SPDX/copyright notice, source link and all transitive dependency/asset licenses before merging. Copy upstream's notice/LICENSE text as required. Do not imply this project authors own third-party source.

External operator-installed Java JARs are not bundled and preserve their own copyrights/licenses.
