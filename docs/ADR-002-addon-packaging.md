# ADR 002: shared addon source, explicit client packages

Date: 2026-11-03. Status: accepted direction; package implementation and manager qualification pending.

## Evidence

- CurseForge documents multi-TOC packaging: https://support.curseforge.com/support/solutions/articles/9000209856-multi-toc-for-world-of-warcraft-addons
- BigWigs packager supports multi-TOC, comma-separated interfaces, conditional TOC loading and explicit target builds: https://github.com/BigWigsMods/packager
- Its release.sh currently maps forever/camelot to the Forever flavour, recognises 16xxx interfaces and emits `_Camelot.toc` in its splitting path. Some enumerations do not list `_Forever.toc`; test generated archives rather than assuming suffix equivalence: https://github.com/BigWigsMods/packager/blob/master/release.sh
- WowUp requires tagged GitHub releases containing packaged addon ZIPs; ZIP root contains the addon directory: https://wowup.io/guide/get-addons/overview and https://wowup.io/guide/wowup/hub
- Verified actual WowUp.Addon release 1.0.30 includes `WowUp-1.0.30.zip` and `release.json`: https://github.com/WowUp/WowUp.Addon/releases/tag/1.0.30
- Author-measured early Forever beta evidence reports modern client APIs and Retail project identity despite interface 16001. This is dated evidence, not proof of current build behaviour: https://github.com/Thunderz96/forever-addon-kit
- wowlua offers typed Lua, multi-flavour checking/builds and simulation: https://wowlua.dev/ . Candidate development aid only, not adopted or a substitute for live validation.

## Decision

Keep one repository, shared plain Lua core and three adapter subdivisions. Classic must additionally name the variant (Era/progression etc.). Separate game family, variant/channel, interface version, build and addon release version. Never infer Retail APIs from interface >= 100000 or identify Forever solely by WOW_PROJECT_ID. Use explicit packaging plus feature checks and runtime evidence.

Ship distinct addon archives initially, each containing `AzerothOperator/` directly at ZIP root and a matching base TOC with the exact qualified interface. Shared source is staged into these artifacts, not forked. Distinct archives make the qualification boundary clear while avoiding uncertain newer TOC suffix handling. Multi-TOC universal packaging can follow after manager/client tests.

Use pinned BigWigs packager for release ZIPs and host metadata; `.pkgmeta` allowlists addon source and excludes Rust binaries, credentials, local snapshots, tests and development files. Pin dependencies and tool revisions. Do not publish untested interface versions merely to silence out-of-date warnings.

Release Rust companion separately as Windows executable/archive with checksums. Never put it in the WoW addon ZIP. Publish GitHub prereleases first; distribute through addon managers only after proving their flavour detection with the actual release metadata. CurseForge/Wago publication requires configured project IDs and credentials; none are assumed.

Pipeline: source checks -> Rust and Lua tests -> per-flavour staging -> TOC/load-order/ZIP-content validation -> fixture runtime -> actual client export -> import/MCP verification -> qualified release. Full client restart for first installation or TOC changes; user-triggered reload for ordinary Lua iteration. Installation updates only our addon folder, preserves WTF/SavedVariables and retains a rollback copy. No computer use or automatic game input.

## Corrections to current prototype

Forever adapter's hardcoded beta variant is provisional, not inferred truth from `_forever_`. Packaging must supply explicit variant/channel and runtime interface validation. The current basic collectors are not all complete datasets: visible currencies and achievement points must remain labelled partial. Lua fixture success is not API compatibility evidence for any client.

## Qualification gaps

Local installation metadata shows Retail 12.1.5.70688 and Forever 1.60.1.70755. No Classic directory was observed in the named WoW installation. No packages or manager installation tested yet. No source-only ZIP advertised as installable, no release publication performed by this decision.
