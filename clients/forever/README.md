# Forever

Owns Forever addon code, API adaptation, capability inventory and runtime qualification.

See the [interaction map](INTERACTIONS.md) for implemented coverage, planned analysis and actions that stay with the player.

## Launch checkpoint, 7 November 2026

The local `_forever_` installation identifies itself as product `wowf`, build `1.60.1.70862`. The build probe does not establish the interface number. Interface `16001` is the package candidate based on the prior fixture and existing Forever addon TOCs. The packaging command writes the requested variant into the generated adapter, so the launch package uses `live` even though the checked-in early adapter says `beta`.

The `0.20.0-dev` Forever package passed byte and manifest verification, a packaged Lua 5.1 export/import round trip and a changed-interface rejection. Source base `4118e51`, SHA-256 `1655f34082acab17adff1166def7315f5fdbcee418492af212f76e3b0eb558f8`. Azeroth Operator was absent from the Forever installation. No installation, game input or native qualification was performed.

## In-game coverage

### Capture-only installation checkpoint

Forever `0.27.0-dev` contains only the adapter, collectors, export core and TOC. The ZIP SHA-256 is `e6190ec5842b0d273a029b7e87bb57ab5622890d9dff78104f40a2f1bd87a803`. The exact package passed a Lua export/import round trip, interface mismatch refusal and absence of the Apply handler. Retail and Classic package entries were verified byte-identical against the previous packager.

Package verification now runs the detailed Forever fixture against the actual staged Lua files through the registered `/aoexport` command. This covers banks, selected recipes, faction standings, currencies, tracked achievements and quest progress, including malformed and unavailable cases. The same ZIP was first-installed and hash-verified on a synthetic Windows client. Retail and Classic fixture packages passed the unchanged verification path. These are offline checks.

The Windows release companion imported the complete synthetic Forever fixture. Its executable SHA-256 `aae7f17c0762d54a42ff76cc92aad4dbb5f27d7b00d84577931d92db6ba86550` was verified and selected in the existing sync configuration. The previous configuration is backed up in `local-data/sync/config-before-forever-capture.json`. The scheduled runner matches its source and ran at 16:15:41 SAST on 7 November 2026 with result 0. Forever returned `waiting_for_export` and Retail returned `unchanged` from its cached acknowledgement. The task reported no live game access.

The staged 0.28.0-dev package adds fixed-material review for selected enchant recipes using the existing snapshot format. SHA-256 `164c6442250d4f1a85c656c231de6e89bfe0c05aad91528ac349f3b2aaa9539a`. Packaged Lua round trip, changed-interface refusal and the synthetic Windows installer test passed offline.

After closing Forever, first-install the reviewed capture package from the repository root:

```powershell
$manifest = (Resolve-Path 'dist/forever-enchant-capture/AzerothOperator-0.28.0-dev-forever-live.manifest.json').Path
python -m deploy.install_forever_capture --client 'T:/Games/World of Warcraft/_forever_' --manifest $manifest --expected-build 1.60.1.70862
```

The installer checks the build and package against the local source, publishes all four files together, and refuses an existing addon directory. Then start Forever, use `/aoexport`, and reload or log out to persist the snapshot for sync. A changed client build needs fresh qualification.

Use trusted local directories. If verification fails after publication, the installer reports that the directory was created and preserves it for inspection. The Windows synthetic CLI test checked wrong-build and corrupt-package refusal, exact installed hashes, existing-file preservation and post-publication failure reporting. Caller and failure review covered the Windows rename and package checks. Jev advisory triage supported the trusted-local recovery boundary (`jev-1.14.0`, request `7fff9f320321b40de5c142e6f31a9ef9a03d843d13210bfa345849dd6b2929cb`, 442 input/44 output tokens, 0.279 seconds).

Caller and failure review checked package contents, installed-file drift, command import resolution and interrupted staging. The companion was verified before its atomic move and configuration selection. Jev advisory triage marked the cutover issues material (`jev-1.14.0`, request `58e7e1f9bc6b982a96ef0c0e6d86d52e75323d6d7c7c9b02108e1188d552c3d3`, 431 input/46 output tokens, 0.294 seconds). Forever was running and its addon directory was absent. The package was staged in the project. Installation, restart and native capture remain pending.

### Coverage by system

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

Set the same environment variable in the local stdio MCP client's server configuration. The optional `lookup_forever_catalogue` tool reads that file only. For example, use `{"section":"items","key":"744"}` for an item, `{"section":"npcs","key":"ragna"}` for a name search, or `{"section":"encounters","key":"Default/Raid Target"}` for a simulator encounter preset. Omit all filters for a section count. Lookups return at most 20 matches and `has_more` flags truncation.

The sync command uses curl for a bounded HTTPS download, checks the SHA-256 pinned in `catalogue.lock.json` and replaces the destination atomically after verification. A matching cache skips the network. Upgrading data requires reviewing and updating the revision and hash in the pin, rebuilding the companion, and rerunning sync. The catalogue includes simulator reference zones, NPCs and factions. For a wider world mirror, qualify an exportable Forever source independently for coverage, build identity and redistribution terms before adding it.

For crafting references, pass `{"section":"items","recipe_spell_id":2675}` to find catalogue items associated with that recipe. The `enchants` section supports the same filter for enchant references. A supplied `key` further narrows the matches. These associations come from the pinned simulator catalogue and can be incomplete or list several variants. Use the saved recipe observation for reagent requirements.

For a set reference, pass `{"section":"items","set_id":495}`. Compare those catalogue item IDs with `review_forever_owned_gear` using the same `set_id`. The result covers membership in the pinned catalogue. Set bonuses, acquisition and pieces in uncaptured storage need separate evidence.

Verified on 7 November 2026: the actual sync command returned `synced` then `unchanged`; a bad download preserved the destination. The Rust stdio MCP returned exact pinned records across all nine lookup sections. The unconfigured server retained 14 tools. Caller and failure review caught the lack of a total HTTP deadline in the initial implementation. The revised command has a 30-second transfer deadline and a 35-second process timeout. Jev advisory triage classified the finding as material (model `jev-1.14.0`, request `b3f8fe8728dbc6193bf06ddff54922a80d21c1f0980322deea9af094d055e3fb`, 422 input/46 output tokens, 0.304 seconds).

### Simulation and optimisation

**Owned gear capture.** The Forever exporter now records character-bank and account-bank containers separately when the bank is open and each visible bank's tab and free-slot counts reconcile. It retains item links and available GUIDs alongside the occupied slots. A closed or unreadable bank is unavailable, while an observed empty bank is reported as zero. The Rust MCP exposes both sections through `inspect_snapshot` and counts through `character_summary`. The updated reader and importer must be in place before using this addon with the hosted sync path.

With the local catalogue configured, call `review_forever_owned_gear` with a saved `snapshot_id` and optional `offset` and `limit` (1–50). It reports storage coverage and joins each observed item location to its base catalogue ID. It preserves observed item links, distinguishes unmatched IDs, and reports totals across all pages. Matched base records include available hand and equipment types, level and class restrictions, uniqueness and limit categories, set identifiers and possible random suffixes. These use simulator catalogue IDs. A null field means the catalogue omitted it. It does not treat a base-ID match as exact modifier data, equip legality or a ranking. Use the optional `item_type` filter, such as `finger`, `trinket` or `weapon`, to page through that catalogue category. Use `set_id` to find observed pieces of a specific catalogue set. Both filters can be combined. Each filter excludes locations without matching catalogue metadata. `filtered_item_locations` and `has_more` refer to the filtered results. Storage coverage and base match totals describe the full observation. Bank APIs still require live qualification. A zero-capacity purchased container currently makes that bank unavailable until native behaviour can distinguish an unequipped slot from an unloaded one.

Offline verification used a synthetic Forever bank export, exact name/realm lookup and an actual stdio MCP call against the pinned catalogue. It checked missing and empty banks, malformed item metadata, cross-storage overlap, same-base-ID variants, pagination and owner scoping. The Forever `0.21.0-dev` candidate passed package round-trip and interface mismatch checks (SHA-256 `cd5770c8411113c6efcb5fc172bdf2e2bc549573b8312fb285ecbb1f5dcd3012`). It was not installed.

**Character selection.** An explicitly requested character overrides the default. Use `list_snapshots` with `{"family":"forever","character":{"name":"Name","realm":"Realm"}}` to resolve saved snapshots for that character. Without an explicit target, use the currently played character only when a fresh, verified identity signal exists. SavedVariables persist on reload or logout, so the latest snapshot alone cannot establish who is in play. Automatic active-character resolution is pending.

**Profession planning.** Select a recipe in the player's own Forever crafting panel, then export a snapshot. Once imported, `review_forever_recipe` accepts its `snapshot_id` and `crafts` (1–1000). The MCP multiplies fixed required reagents by the requested craft count and reports observed stock, shortfall and uncaptured storage. Unsupported required slots suppress the estimate. Native capture is pending qualification. Use the updated companion and importer before sending the new schema through hosted sync.

- **Simulation.** [WoWSims Forever](https://github.com/wowsims/forever) revision `9dc32ac8ca0d8d1b76337861d38b93c0f884af11` built on Windows with Go 1.27.3, protoc 36.2, protoc-gen-go 1.36.10 and the upstream embedded database. A local SIML run used the upstream Survival Melee Hunter build over the Hunter defaults, including its explicit melee rotation, gear, raid buffs and one-target 180-second encounter. At seed 20261107 and 100 iterations, five independent CLI invocations agreed after sorting spell rows. Mean DPS was 770.047468 with standard deviation 36.709963. All inputs were public upstream fixtures. The MCP still needs a validated mapping from captured characters and owned items to simulator requests before it can rank legal sets.
- **Catalogue.** The [pinned WoWSims database](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/assets/database/db.json) parsed locally with 6,043 distinct item IDs. Only 3,945 have source records, and all carry phase value 1. SHA-256 `5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e`. Use item data for simulator compatibility and provenance-aware lookup. Use separately verified acquisition evidence such as [ForeverDiff](https://foreverdiff.gg/) and official unlocks for obtainable-gear claims. Blizzard lists the first three raids for 9 December. [Blizzard raid overview](https://worldofwarcraft.blizzard.com/en-us/news/24303009/world-of-warcraft-forever-raid-overview-locations-raid-sizes-and-attunement)
- **Optimisation.** Keep three candidate sets explicit: equipped gear, owned gear and obtainable upgrades. Filter by level, class, faction, weapon and off-hand rules, unique-equipped limits, profession and content availability. Evaluate full sets so set bonuses, caps, enchants, suffixes and procs can affect the result. Label unobserved inventory and unknown acquisition separately. Report the baseline, assumptions, coverage, tested candidate count and uncertainty with rankings. A bounded search result applies only to the evaluated candidates and encounter.

Run personal profiles locally by default. Keep provider revisions and data build claims separate from the character snapshot build. Preserve the distinction between a simulated recommendation and an in-game equipment change.
