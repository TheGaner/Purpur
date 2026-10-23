# License and import policy

This Rust-only branch's original server, Java-bridge source and example plugins are licensed AGPL-3.0-or-later (full text in LICENSE); Cargo.toml contains the corresponding SPDX identifier. This does not change the upstream Purpur Java branch's MIT license. Fresh orphan branch code currently does not copy implementation from the other projects listed below.

Review of upstream license files on 2026-10-23:
- PurpurMC/Purpur: MIT; license says Copyright 2019–2024 PurpurMC.
- ludovic111/horizon: MIT; Copyright 2026 Ludovic.
- LuckPerms/LuckPerms: MIT; copyright lucko and contributors.
- ferrumc-rs/ferrumc: MIT; copyright 2024 Saad Muhammad.
- Steel-Foundation/SteelMC: AGPL-3.0-or-later.
- EssentialsX/Essentials: GPL version 3 license text; each imported file's exact version/exception must be checked.
- PaperMC/Folia: GPL version 3 patch license; underlying inherited source may have separate licensing.

AGPL is the conservative first-party project choice because later direct worldgen reuse from SteelMC is planned. Copying SteelMC code needs version-pinned provenance, notices and compliance including AGPL network-source obligations. Permission to copy MIT-licensed components requires preserving the copyright/license notices. GPLv3 and AGPLv3 have a special combination provision (section 13), not permission to erase original GPL obligations or put GPL-only copied files under AGPL alone. Distinguish a standalone separately licensed JAR from directly incorporated Java code; process separation alone is not proof of legal independence.

Do not copy proprietary Minecraft server code, proprietary game data/assets, or decompiled code without an independent rights assessment. License and test every imported code/data dependency individually. Native plugins have full process permissions; the Java sidecar is not a security sandbox. Third-party plugins keep the original authors' licenses. Maintain a public matching-source offer and build reproducibility for AGPL deployments. Substantial GPL+AGPL combination or distribution should get qualified legal review. This is an engineering compliance plan, not legal advice.
