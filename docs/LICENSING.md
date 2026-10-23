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


## Native EssentialsX/LuckPerms integration and combination procedure (October 23, 2026)

Decision: The server ships original Rust implementations of essential utilities and permission services and enables them automatically. There is no required Java plugin, embedded EssentialsX jar or LuckPerms jar. Initial modules under src/builtin are original code inspired by functionality, NOT translated upstream implementation, not upstream-affiliated, and not complete behavioral parity.

### Direct source compatibility
- LuckPerms MIT source: may be copied, adapted and distributed with required original copyright and MIT license notices intact; record per-file provenance. The copied MIT code retains its authorship and licensing obligations even inside an AGPL-distributed server.
- EssentialsX GPL version 3 source: GPLv3 section 13 and AGPLv3 section 13 expressly allow a combination of GPLv3-covered and AGPLv3-covered work. The GPL terms continue for the GPL-covered part; AGPL network-source rules apply to the combination. Do not substitute AGPL-only headers on directly GPL-derived source. Audit exact per-file terms (including potential exceptions) before copying or porting. Independently implemented equivalent behaviors do not automatically make the new Rust source a copied derivative.
- SteelMC AGPL-3.0-or-later source: later worldgen reuse must preserve copyright and license notices, dependency/generated asset terms and matching source.
- Serde and serde_json direct Cargo dependencies declare MIT OR Apache-2.0; exact locked versions and transitive license inventory are a release prerequisite.
- Upstream brand names must not imply an official EssentialsX/LuckPerms release or endorsement.

### Must-do steps BEFORE copying implementation source

1. Pin exact upstream SHA, inspect source copyright and license headers, contributor files, generated assets, dependency manifests and any exceptions.
2. Choose per feature: independently authored Rust behavior, MIT-attributed adaptation, or GPLv3/AGPLv3 attributed direct port. Record rationale and boundary.
3. Check exact compatibility of ALL imported terms; GPLv3 and AGPLv3 combine under respective section 13 without extinguishing GPL rights/obligations. Obtain legal review when mixing substantial copied GPL sources.
4. Preserve SPDX/license headers in copied files, upstream copyright notices and original license texts. Append exact upstream URL, SHA, source/destination paths and modifications to THIRD_PARTY_NOTICES.md.
5. Verify behavior with version-pinned golden data and reference tests. Do not equate a compiling port with full upstream compatibility.
6. Produce locked dependency license scan, transitive source/asset SBOM, matching corresponding source, reproducible build instructions and notices in each distributed release.
7. For AGPL-covered modified network service versions, implement a prominent no-charge Corresponding Source retrieval offer, e.g. /source command and published immutable source archive matching deployed build. A generic source landing page alone may not satisfy this requirement.
8. Re-audit changes on every upstream upgrade. Preserve authored Rust files' own AGPL grant and third-party component licenses separately.

Primary sources: https://www.gnu.org/licenses/gpl-3.0.html#section13 ;
https://www.gnu.org/licenses/agpl-3.0.html#section13 ;
https://www.gnu.org/licenses/gpl-faq.html ;
https://github.com/EssentialsX/Essentials/blob/2.x/LICENSE ;
https://github.com/LuckPerms/LuckPerms/blob/master/LICENSE.txt .

No directly copied implementation files are included from EssentialsX, LuckPerms or SteelMC at this stage, therefore the GPL combination procedure is a future import gate rather than an assertion a covered-source merge has already occurred. This is an engineering compliance checklist, not legal advice.
