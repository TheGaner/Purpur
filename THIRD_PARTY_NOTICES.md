# Third-party attribution and provenance

No implementation files from the named upstream projects are currently copied into this Rust-only server branch. This repository is a fork of the Purpur GitHub repository, but horizon-rust begins as an independent orphan Rust tree; original Java Purpur remains on ver/26.3.

| Upstream | License observed | Code imported |
| --- | --- | --- |
| https://github.com/PurpurMC/Purpur | MIT | None |
| https://github.com/ludovic111/horizon | MIT | None |
| https://github.com/Steel-Foundation/SteelMC | AGPL-3.0-or-later | None (worldgen planned) |
| https://github.com/LuckPerms/LuckPerms | MIT | None |
| https://github.com/ferrumc-rs/ferrumc | MIT | None |
| https://github.com/EssentialsX/Essentials | GPL v3 | None (native original implementation; optional Java compatibility) |
| https://github.com/PaperMC/Folia | GPL v3 patches and inherited licenses | None (region scheduler design only) |

Future source imports MUST add an immutable upstream commit SHA, exact imported paths and modifications, SPDX/copyright notice, source link and all transitive dependency/asset licenses before merging. Copy upstream's notice/LICENSE text as required. Do not imply this project authors own third-party source.

External operator-installed Java JARs are not bundled and preserve their own copyrights/licenses.


### New native persistence dependencies

Serde and serde_json are first-party build dependencies for JSON persistence. Their package manifests state MIT OR Apache-2.0. Check Cargo.lock and transitive dependencies, generate an SBOM, and distribute notices/license texts before a public binary release. These crates are not from the EssentialsX or LuckPerms repositories.


## Locked Cargo registry dependency declarations (2026-10-23)

This is the checked, reproducible dependency **declaration** inventory for
the Rust-only branch at the current Cargo.lock. All packages are downloaded
via the crates.io registry; version and checksum are pinned in
[docs/CARGO_DEPENDENCY_LICENSES.json](docs/CARGO_DEPENDENCY_LICENSES.json).
No upstream server implementation code is imported by these dependencies.

| Crate | Locked version | License declared by Cargo package manifest |
| --- | --- | --- |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| zmij | 1.0.23 | MIT |

Run python3 scripts/check_cargo_licenses.py using the pinned local Rust
1.90.0 toolchain to detect changes in dependency names, versions, source,
checksums, and license expressions. If it reports drift, review each new
or changed upstream license and its actual text before explicitly refreshing
the snapshot with --accept-current. Re-run the checker after each lockfile
update and include this inventory in release review.

**Not yet satisfied:** Package metadata is not a per-file copyright,
generated-asset or nested-license audit. The complete redistributable
license texts and third-party copyright notices, chosen license alternatives,
matching source archives and a prominent AGPL network source offer must be
assembled and validated before shipping any binaries. This table and the
checker are NOT a legal compliance certificate or permission to import source.
