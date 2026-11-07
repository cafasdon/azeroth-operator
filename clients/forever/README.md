# Forever

Owns Forever addon code, API adaptation, capability inventory and runtime qualification.

## Launch checkpoint, 7 November 2026

The local `_forever_` installation identifies itself as product `wowf`, build `1.60.1.70862`. The build probe does not establish the interface number. Interface `16001` is the package candidate based on the prior fixture and existing Forever addon TOCs. The packaging command writes the requested variant into the generated adapter, so the launch package uses `live` even though the checked-in early adapter says `beta`.

The `0.20.0-dev` Forever package passed byte and manifest verification, a packaged Lua 5.1 export/import round trip and a changed-interface rejection. Source base `4118e51`, SHA-256 `1655f34082acab17adff1166def7315f5fdbcee418492af212f76e3b0eb558f8`. Azeroth Operator was absent from the Forever installation. No installation, game input or native qualification was performed.

## In-game coverage

| Area | Current Forever support | Evidence needed |
| --- | --- | --- |
| Equipment, bags, character stats, basic talents, macros, bindings, professions, quests, currencies and achievements | Shared export candidates, with explicit unavailable sections when a collector fails | One real `/aoexport`, reload and MCP readback. Record the exact build/interface, missing sections and partial coverage |
| Item instance identity, equipment sets, binding context, action slots and talent loadout strings | Important observations are Retail-gated or absent | Confirm Forever APIs and data shape before enabling each collector. Include bag and bank visibility, spell ranks, weapon skills and talent ranks in the inventory |
| Player-applied macros, bindings, actions, equipment, bags and talents | Forever execution disabled | Verify exact preimages and postconditions operation by operation on Forever, then add explicit player review, one Apply and a receipt. Keep combat and protected-action guards |
| Gameplay actions and transactions | Outside the current automation boundary | Offer read-only planning where supported |

The first native milestone is a persisted read-only snapshot and a coverage report through MCP. Record what the actual client returns before adapting any Retail operation.

## External data and gear

### Local catalogue

Run from the repository root:

```powershell
python -m tools.sync_forever_catalogue --output local-data/forever/catalogue.json
# Set AZEROTH_DATA_DIR to your existing private, absolute snapshot directory.
$env:AZEROTH_FOREVER_CATALOGUE = (Resolve-Path local-data/forever/catalogue.json).Path
.\target\debug\azeroth-operator.exe
```

Set the same environment variable in the local stdio MCP client's server configuration. The optional `lookup_forever_catalogue` tool reads that file only. For example, use `{"section":"items","key":"744"}` for an item, `{"section":"npcs","key":"ragna"}` for a name search, or `{"section":"encounters","key":"Default/Raid Target"}` for a simulator encounter preset. Omit `key` for a section count. Names return at most 20 matches and `has_more` flags truncation.

The sync command uses curl for a bounded HTTPS download, checks the SHA-256 pinned in `catalogue.lock.json` and replaces the destination atomically after verification. A matching cache skips the network. Upgrading data requires reviewing and updating the revision and hash in the pin, rebuilding the companion, and rerunning sync. The catalogue includes simulator reference zones, NPCs and factions. For a wider world mirror, qualify an exportable Forever source independently for coverage, build identity and redistribution terms before adding it.

Verified on 7 November 2026: the actual sync command returned `synced` then `unchanged`; a bad download preserved the destination. The Rust stdio MCP returned exact pinned records across all nine lookup sections. The unconfigured server retained 14 tools. Caller and failure review caught the lack of a total HTTP deadline in the initial implementation. The revised command has a 30-second transfer deadline and a 35-second process timeout. Jev advisory triage classified the finding as material (model `jev-1.14.0`, request `b3f8fe8728dbc6193bf06ddff54922a80d21c1f0980322deea9af094d055e3fb`, 422 input/46 output tokens, 0.304 seconds).

### Simulation and optimisation

**Owned gear capture.** The Forever exporter now records character-bank and account-bank containers separately when the bank is open and each visible bank's tab and free-slot counts reconcile. It retains item links and available GUIDs alongside the occupied slots. A closed or unreadable bank is unavailable, while an observed empty bank is reported as zero. The Rust MCP exposes both sections through `inspect_snapshot` and counts through `character_summary`. The updated reader and importer must be in place before using this addon with the hosted sync path.

**Character selection.** An explicitly requested character overrides the default. Use `list_snapshots` with `{"family":"forever","character":{"name":"Name","realm":"Realm"}}` to resolve saved snapshots for that character. Without an explicit target, use the currently played character only when a fresh, verified identity signal exists. SavedVariables persist on reload or logout, so the latest snapshot alone cannot establish who is in play. Automatic active-character resolution is pending.

- **Simulation.** [WoWSims Forever](https://github.com/wowsims/forever) is the first engine candidate. Pinned source `9dc32ac8ca0d8d1b76337861d38b93c0f884af11` contains a CLI that accepts a RaidSimRequest JSON or SIML setup. Its source warns that browser-generated rotations without explicit APL rows can run without a rotation. The latest-release endpoint returned 404 and Go was unavailable in the checked Windows and Debian shells. No simulation run was verified. First acceptance check is a supported spec and synthetic baseline with explicit APL, encounter, buffs, duration, iterations and seed, compared against the same request in the upstream engine.
- **Catalogue.** The [pinned WoWSims database](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/assets/database/db.json) parsed locally with 6,043 distinct item IDs. Only 3,945 have source records, and all carry phase value 1. SHA-256 `5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e`. Use item data for simulator compatibility and provenance-aware lookup. Use separately verified acquisition evidence such as [ForeverDiff](https://foreverdiff.gg/) and official unlocks for obtainable-gear claims. Blizzard lists the first three raids for 9 December. [Blizzard raid overview](https://worldofwarcraft.blizzard.com/en-us/news/24303009/world-of-warcraft-forever-raid-overview-locations-raid-sizes-and-attunement)
- **Optimisation.** Keep three candidate sets explicit: equipped gear, owned gear and obtainable upgrades. Filter by level, class, faction, weapon and off-hand rules, unique-equipped limits, profession and content availability. Evaluate full sets so set bonuses, caps, enchants, suffixes and procs can affect the result. Label unobserved inventory and unknown acquisition separately. Report the baseline, assumptions, coverage, tested candidate count and uncertainty with rankings. A bounded search result applies only to the evaluated candidates and encounter.

Run personal profiles locally by default. Keep provider revisions and data build claims separate from the character snapshot build. Preserve the distinction between a simulated recommendation and an in-game equipment change.
