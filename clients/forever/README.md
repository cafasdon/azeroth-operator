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

Set the same environment variable in the local stdio MCP client's server configuration. The optional `lookup_forever_catalogue` tool reads that file only. For example, use `{"section":"items","key":"744"}` for an item, `{"section":"npcs","key":"ragna"}` for a name search, or `{"section":"encounters","key":"Default/Raid Target"}` for a simulator encounter preset. Omit all filters for a section count. Lookups return at most 20 matches. If `has_more` is true, keep the same filters and advance `offset` by 20.

The sync command uses curl for a bounded HTTPS download, checks the SHA-256 pinned in `catalogue.lock.json` and replaces the destination atomically after verification. A matching cache skips the network. Upgrading data requires reviewing and updating the revision and hash in the pin, rebuilding the companion, and rerunning sync. The catalogue includes simulator reference zones, NPCs and factions. For a wider world mirror, qualify an exportable Forever source independently for coverage, build identity and redistribution terms before adding it.

For crafting references, pass `{"section":"items","recipe_spell_id":2675}` to find catalogue items associated with that recipe. The `enchants` section supports the same filter for enchant references. A supplied `key` further narrows the matches. These associations come from the pinned simulator catalogue and can be incomplete or list several variants. Use the saved recipe observation for reagent requirements.

Use `{"section":"items","source_kind":"pvp"}` to browse PvP-tagged items. The other source categories are `crafted`, `drop`, `quest`, `vendor` and `reputation`. Filters combine at item level, so an item can match through multiple recorded sources. The catalogue PvP tag supplies no rank, cost or season data.

For recorded loot sources, pass `{"section":"items","drop_npc_id":644}`. Resolve NPC names through the `npcs` section first. Results include only items with an explicit matching NPC drop in the pinned catalogue. For zone loot, use `drop_zone_id` to include both NPC and unattributed drops recorded for that zone. When both filters are supplied, they must match the same drop source. Missing entries provide no evidence about live loot availability.

For vendor references, pass `{"section":"items","vendor_npc_id":1297}`. Matching sale records can include the vendor name, zone and faction. Current stock, prices and purchase eligibility require separate evidence.

For reputation rewards, pass `{"section":"items","reputation_faction_id":509}` using a reputation faction ID from the saved standings or the catalogue. Source records can include a required reputation tier and an Alliance or Horde restriction. Character eligibility requires separate verification.

For quest rewards, pass `{"section":"items","reward_quest_id":53}` using the quest ID from a saved quest observation or other reference. Results cover reward associations recorded in the pinned catalogue. Character eligibility and live availability require separate evidence.

For a public set reference, pass `{"section":"items","set_id":495}`. For a character comparison, call `review_forever_owned_gear` with the same `set_id` and a saved snapshot ID. Its `set_observation` lists each catalogue piece, whether its base ID was observed, and the number of occupied locations by storage. Duplicate copies count once toward distinct pieces. The summary spans the whole snapshot regardless of page, item ID or equipment category. Use storage coverage to see which sections contributed. An unseen piece may be in uncaptured storage. Set bonuses, acquisition and pieces in uncaptured storage need separate evidence.

Verified on 7 November 2026: the actual sync command returned `synced` then `unchanged`; a bad download preserved the destination. The Rust stdio MCP returned exact pinned records across all nine lookup sections. The unconfigured server retained 14 tools. Caller and failure review caught the lack of a total HTTP deadline in the initial implementation. The revised command has a 30-second transfer deadline and a 35-second process timeout. Jev advisory triage classified the finding as material (model `jev-1.14.0`, request `b3f8fe8728dbc6193bf06ddff54922a80d21c1f0980322deea9af094d055e3fb`, 422 input/46 output tokens, 0.304 seconds).

### Simulation and optimisation

The standalone `python -m tools.run_forever_sim` command combines saved
equipment and encounter MCP responses with a trusted local WoWSims executable
and a caller-supplied `RaidSimRequest` JSON template. Supply `--engine` as an
absolute path, `--engine-sha256` from the trusted local build, and the three
input paths through `--template`, `--equipment-review` and
`--encounter-lookup`. Save the JSON text inside each MCP response as its own
file. The template uses canonical lower-camel-case protobuf JSON field names and supplies one
player with class, race, spec, talents, explicit APL, and the desired buffs
and consumables. Disable item swapping. The equipment and encounter exports
replace their corresponding template inputs.

The command runs locally with a 120-second timeout and prints only the DPS
summary, iteration count, seed and executable checksum. The supplied checksum
must come from a build you trust. Temporary request and result files are
removed after the run. The template is explicit caller input, so results only
represent the captured character when the template accurately describes that
character. This command is separate from the MCP tool registry.

Repeat `--equipment-review` to compare up to eight drafts against the same
template and encounter. The first input is the baseline. Multi-review output
keeps the original order and gives each result a one-based review index and
its mean DPS delta from the baseline. Identical assembled requests run once
within the comparison. The batch shares a 120-second simulation deadline and
allows at most 100,000 requested iterations across distinct requests. Mean
deltas are descriptive by default. Add `--paired` with 2–5,000 iterations to
report the standard error of each DPS difference against the baseline. This
pairs iteration samples using the same seed, template and encounter, and keeps
the raw samples out of the output. It does not search the full owned inventory.

`python -m tools.rank_forever_slot` ranks gear across any of the 17 simulator slots. Use the same
engine, template and encounter arguments, plus `--mcp` for the absolute path
to the trusted local MCP executable and `--snapshot-id`. Configure
the usual owner snapshot directory and pinned catalogue in the environment.
Repeat `--slot` or use `--all-slots` to compare combinations, including keeping the
current item in each slot. The command pages up to 256 matching captured locations
per slot and scans at most 1,024 combinations, including the baseline. It skips
combinations that reuse a physical source or leave a moved item's slot unfilled.
It checks up to 64 remaining changes, rejects larger searches and sorts resolved
results by mean DPS against the captured baseline.
`--paired` adds uncertainty against that baseline. Results include storage
coverage and skipped candidates. Multi-slot output uses `slots` and a
`selections` array per result. Character eligibility and complete-inventory
optimisation remain separate checks.

Repeat `--item-id` to restrict alternatives to up to 16 distinct base item
IDs. The filter is applied before paging. The equipped baseline still uses
the complete captured equipment. The result applies only to the requested
IDs in the selected slots and the observed storage. MCP callers can use
`item_ids` on `review_forever_owned_gear` instead of the single-ID
`item_id` option.

The ranker checks the template's class and race against the saved stats when
it recognises the captured tokens. A known mismatch stops the run. Missing
or unmapped observations are reported as `unverified` in `template_identity`.
The check uses the pinned simulator enum mapping. The simulator request also
requires the template's specialisation to agree with its declared class.
[WoWSims Forever enum definitions](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/common.proto)

The MCP also excludes a proposed swap from simulation when the pinned item
record explicitly requires a higher level or excludes the captured class.
Slot-filtered owned-gear reviews expose the conflict before the ranker expands
combinations. Draft reviews and skipped ranker rows report the conflicting slots. These
checks use available snapshot stats and catalogue records. Full in-game
eligibility remains unverified.

The ranker also skips locations whose saved links cannot produce a simulator
item input before expanding combinations. Full drafts still go through the
MCP because slot interactions can affect whether a complete set is usable.

When the item requires a mapped profession and a professions section was
captured, the check also excludes swaps if that skill line is absent.
It reports the required skill-line ID. Missing profession observations leave
this check unknown. The pinned item catalogue supplies the profession, with
no required skill rank on these gear records.
[Pinned WoWSims profession mapping](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/tools/database/dbc/maps.go)

New Forever exports include the player faction in `stats` when the client
returns a recognised faction tag. The MCP uses it to exclude explicit
opposite-faction catalogue gear. Older captures or unavailable faction leave
that check unknown. Use a compatible importer and reader before syncing the
new field. [Forever Unit API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitDocumentation.lua)

Use `--collapse-identical-items` to collapse resolved simulator-equivalent
storage copies before applying the combination and draft limits. This applies
to items that fit exactly one simulator slot. Each skipped copy records its
representative location. Equipped items and items that can fit multiple slots
keep their separate locations so a draft can still use two physical copies.

For adaptive bulk ranking, build the local adapter from the pinned WoWSims
checkout with its generated protobuf files and embedded database:

```powershell
go build -tags with_db -o <adapter-path> <absolute-path-to-tools/forever_bulk/main.go>
```

Add `--bulk-engine` with the absolute adapter path, `--bulk-engine-sha256`
with its SHA-256, and optionally `--top` (default five distinct alternatives).
Use a template with its player in the first party. The adapter receives only
distinct, resolved simulator gear sets. The result includes the baseline, the
shortlist with its matching observed selections, and the engine's stage metrics.
An unavailable paired standard error is null. The engine chooses its precision
and iteration counts and may use a single stage for small searches. The
64-draft and 120-second comparison limits apply in bulk mode. The encounter's
requested iteration count and the fixed mode's aggregate iteration cap apply
to fixed mode, including the fallback when every set matches the baseline.

Repeat `--encounter-lookup` up to four times to rank the same saved gear drafts
separately for each encounter. Output uses an `encounters` array in input order,
with a one-based encounter index, duration, duration variation and target count.
Review IDs identify the same draft across profiles. A bulk shortlist can omit
different drafts in different profiles. Each comparison has its own iteration
and time limits. The command prints results after all comparisons succeed.

For a combined ranking, repeat `--encounter-weight` once per encounter in input
order. Use positive weights such as `--encounter-weight 70 --encounter-weight 30`.
The ranker normalises the weights and averages each draft's percentage DPS change
from its encounter baseline. This keeps a high-DPS encounter from dominating
solely because of its DPS scale. The combined view requires fixed mode so every
draft is evaluated in every encounter, and each baseline must have positive DPS.
It reports weighted point estimates with the original per-encounter results.

To build an encounter and iteration input, call `lookup_forever_catalogue`
with `section:"encounters"`, the exact preset path as `key`, and a
`simulation` object. For example:

```json
{"section":"encounters","key":"Default/Raid Target","simulation":{"duration_seconds":180,"duration_variation_seconds":5,"execute_proportion_20":0.2,"execute_proportion_35":0.35,"iterations":1000,"seed":20261107}}
```

The response includes `simulation_input.encounter` and
`simulation_input.simOptions` in the pinned engine's protobuf JSON format.
The projection keeps preset target order and target inputs. It requires explicit
duration, variation, execute fractions, iteration count and seed. It supplies
inputs only. The preset models and the character's rotation still determine
which mechanics the simulation represents.

The local owned-gear review can return `catalogue_modifier_candidates.simulator_item_spec`, a proto-JSON item input containing the base ID, enchant effect ID and suffix ID. It requires a known simulator slot and resolved captured modifiers. Explicit zeros are preserved. Unknown, ambiguous or unsupported observations return null. This supplies one item input. Complete equipment, character and encounter inputs require separate validation.

An iteration repeats a specified fight with a new random outcome. Keep encounter, rotation, buffs and seed policy fixed when comparing gear. Fight duration, target count and timing, execute windows, movement and downtime can change which set wins when those mechanics are implemented in the engine. Record unsupported mechanics explicitly. Report separate rankings for distinct encounter profiles unless the player supplies weights for combining them. [SimulationCraft raid events](https://github.com/simulationcraft/simc/wiki/RaidEvents)

The pinned Forever bulk engine already has staged culling and reuses samples for survivors. The stages start at 100, 1,000 and 10,000 iterations with target errors of 1%, 0.2% and 0.05%. It can skip staging when requesting every result or when a single pass has a lower estimated cost. Use this engine path for bulk comparisons. Before simulation, reject unsupported or impossible sets and deduplicate identical simulator inputs. Cache by exact engine and catalogue revision, build and rotation, encounter and buffs, gear modifiers and precision settings. Preserve overlapping finalists as sidegrades and compare the staged shortlist with exhaustive high-precision runs on bounded fixtures before tuning culling. [Pinned Forever stage code](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/bulk/stage.go) · [Raidbots Smart Sim](https://support.raidbots.com/article/55-smart-sim)

On Windows, the same public Survival Melee Hunter fixture, 180-second one-target profile and seed 20261107 took 0.271 s at 100 iterations, 1.133 s at 1,000 and 4.944 s at 5,000. Approximate 95% DPS half-widths were 7.20, 1.98 and 0.88 respectively, using 1.96 × sample standard deviation divided by the square root of iterations. These are individual CLI wall-time observations including startup. They measure precision cost for one profile, with no bulk speedup or rank-recall claim.

A second offline check ran the pinned Hunter defaults and `turret.apl.siml` using `siml sim` with `settingsFile`. Copies of the defaults changed only the encounter duration to 45 or 180 seconds and selected either `p1.gear.siml` or `p1_melee.gear.siml`. Both profiles kept one target and five seconds of duration variation. Each gear and duration pair used 1,000 iterations on seeds 20261107–20261109.

| Duration | P1 mean DPS range | P1 Melee mean DPS range | Approximate individual 95% half-widths |
| --- | --- | --- | --- |
| 45 s | 889.702–889.834 | 892.775–892.942 | 3.52–3.58 DPS |
| 180 s | 828.990–829.013 | 831.810–831.854 | 1.66–1.67 DPS |

The duration change shifted both DPS estimates and their precision. The order stayed the same and the individual intervals overlapped in both scenarios. These are per-profile intervals, with no paired-difference estimate or claim that the gear sets are tied. P1 CLI wall time was 0.273–0.309 s for 45 seconds and 0.504–0.519 s for 180 seconds. This synthetic check covers duration only. It adds no evidence about add waves, movement or native encounter accuracy.

An offline bulk pilot exercised the pinned Go `BulkSimAsync` entry point on the same Hunter defaults and turret rotation. It replaced only the chest piece with the first 48 matching catalogue rows in file order. The filter included cloth, leather or mail chest pieces with a level requirement at most 60 and no class, faction or profession restriction, excluding the baseline chest. Both modes used seed 20261107. The isolated harness lowered the stage floors to 40, 160 and 1,000 and set target errors to 5%, 2% and 1%. These are pilot settings.

| Duration | Staged top five | Exhaustive all results | Ordered top-five chest IDs in both modes |
| --- | ---: | ---: | --- |
| 45 s | 0.419 s | 7.135 s | 1981, 4082, 2041, 6773, 7063 |
| 180 s | 1.186 s | 16.761 s | 1981, 4082, 2041, 6773, 7063 |

Staged mode screened 48 candidates at 160 iterations and advanced five to 1,000. Exhaustive mode simulated all 48 at 1,000 and then refined all 48 to 4,000 iterations. The timing compares different search and refinement budgets. Two executions with the same seed reproduced the reported 180-second exhaustive ranking and the 45-second staged ranking. The captured comparison covers one seed, two durations and one changed slot.

The same 45-second case also passed with the pinned engine's unmodified stage floors and target errors. Staged mode screened all 48 at 1,720 iterations, then finished five at 16,203 in 4.470 s. Exhaustive mode ran all 48 at 27,240 and refined all 48 to 108,960 in 156.664 s. Both modes returned the same ordered top five as the reduced-budget pilot. The leading candidate's paired standard error against the next result was 0.090 DPS in staged mode and 0.034 DPS in exhaustive mode. This is one seed and one slot, with different final precision. Multi-slot searches and near-tie recall still need qualification.

With a configured local catalogue, `review_equipment_selection` checks whether a draft leaves an item in the off-hand alongside a catalogue two-handed main-hand item. By default the conflict blocks the simulator equipment spec. For a main-hand selection without an off-hand target, `allow_two_hand_off_hand_clear: true` permits the simulator projection to leave the off-hand empty if needed. The response lists slot 17 in `checks.simulator_cleared_equipment_slots` when that happens. The local ranker enables this for eligible drafts and lists the clear alongside the ranked selection. The draft is read-only and full weapon pairing and game equip eligibility remain unknown.

**Owned gear capture.** The Forever exporter now records character-bank and account-bank containers separately when the bank is open and each visible bank's tab and free-slot counts reconcile. It retains item links and available GUIDs alongside the occupied slots. A closed or unreadable bank is unavailable, while an observed empty bank is reported as zero. The Rust MCP exposes both sections through `inspect_snapshot` and counts through `character_summary`. The updated reader and importer must be in place before using this addon with the hosted sync path.

With the local catalogue configured, call `review_forever_owned_gear` with a saved `snapshot_id` and optional `offset` and `limit` (1–50). It reports storage coverage and joins each observed item location to its base catalogue ID. It preserves observed item links, distinguishes unmatched IDs, and reports totals across all pages. Matched base records include available hand and equipment types, level and class restrictions, uniqueness and limit categories, set identifiers and possible random suffixes. These use simulator catalogue IDs. `simulator_slots` lists slot names from the pinned item type and hand rules. Two-handed items list main hand as their placement. This per-item list does not resolve off-hand conflicts, class restrictions or complete-set legality. Each matched base record also includes the pinned `simulator_default_scaling` record, available weapon speed and recorded item effects. This shows the base values at simulator scaling key zero, separate from the observed link and modifier estimates. A null field means the catalogue omitted it. It does not treat a base-ID match as exact modifier data, equip legality or a ranking. Use the optional `item_type` filter, such as `finger`, `trinket` or `weapon`, to page through that catalogue category. Use `set_id` to find observed pieces of a specific catalogue set. Use `item_id` to find each captured location of a base item, including items absent from the catalogue. The result preserves the observed link for each location. `link_modifiers` reports the raw enchant and random suffix fields when their positions are available and the link base ID agrees with the observation. Empty fields are null, and a zero is the literal encoded value. This reads selected fields only. `catalogue_modifier_candidates` lists exact nonzero ID matches by enchant effect ID or signed suffix ID. Multiple candidates are retained. `default_scaling_suffix_allocation_delta` estimates nonzero regular stat changes using the default simulator scaling and a unique suffix listed for the base item. It reports simulator stat IDs and rounds each stat separately. Missing evidence returns null. The separate `default_scaling_regular_stats_before_enchant` view combines recorded base stats with that allocation, or returns base stats alone when the link encodes suffix zero. An omitted stat map in a present scaling record is treated as empty, as in the simulator. An empty or unresolved suffix field or missing scaling record returns null. The separate `default_scaling_regular_stats_with_enchant` view adds regular stats from exactly one fitting enchant row. An explicitly encoded enchant zero keeps the preceding total. An empty enchant field, no fit or multiple fits returns null. Both totals exclude pseudo-stats, temporary effects and procs. The separate `default_scaling_suffix_extra_effects` field reports nonzero flat bonuses by simulator pseudo-stat ID and, where the suffix selects reduced caster damage, the replacement weapon damage profile. A null weapon profile means the default profile is retained. Empty lists mean no exact match, and a null result means the link fields could not be read. The `fitting_enchants` subset follows the pinned simulator rules for equipment type, hand, weapon subtype and extra enchant types. It is null when the base item has no known catalogue category. This models simulator compatibility. Live eligibility and applied effects require separate evidence. [Pinned simulator fit rules](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/itemfit/itemfit.go) Full link validation, simulator compatibility and live modifier effects remain separate. [Pinned WoWSims exporter field layout](https://github.com/wowsims/exporter/blob/c30fbcc14658e342f084f2dc487dbaa07841c697/ExportStructures/ItemSpec.lua) Filters can be combined. Use `simulator_slot` to filter by a listed slot name such as `off_hand` or `finger2` before paging. Slot, category and set filters require matching catalogue metadata. `filtered_item_locations` and `has_more` refer to the filtered results. Storage coverage and base match totals describe the full observation. Bank APIs still require live qualification. A zero-capacity purchased container currently makes that bank unavailable until native behaviour can distinguish an unequipped slot from an unloaded one.

Offline verification used a synthetic Forever bank export, exact name/realm lookup and an actual stdio MCP call against the pinned catalogue. It checked missing and empty banks, malformed item metadata, cross-storage overlap, same-base-ID variants, pagination and owner scoping. The Forever `0.21.0-dev` candidate passed package round-trip and interface mismatch checks (SHA-256 `cd5770c8411113c6efcb5fc172bdf2e2bc549573b8312fb285ecbb1f5dcd3012`). It was not installed.

**Character selection.** An explicitly requested character overrides the default. Use `list_snapshots` with `{"family":"forever","character":{"name":"Name","realm":"Realm"}}` to resolve saved snapshots for that character. Without an explicit target, use the currently played character only when a fresh, verified identity signal exists. SavedVariables persist on reload or logout, so the latest snapshot alone cannot establish who is in play. Automatic active-character resolution is pending.

**Profession planning.** Select a recipe in the player's own Forever crafting panel, then export a snapshot. Once imported, `review_forever_recipe` accepts its `snapshot_id` and `crafts` (1–1000). The MCP multiplies fixed required reagents by the requested craft count and reports observed stock, shortfall and uncaptured storage. Unsupported required slots suppress the estimate. Native capture is pending qualification. Use the updated companion and importer before sending the new schema through hosted sync.

Use `profession_skill_line` with `compare_snapshots` to see captured rank and
rank cap before and after. It matches the skill-line ID across profession
slots. If either capture lacks an unambiguous row, the rank change is unknown.

`material_capacity` estimates how many crafts the observed fixed reagents
support and lists all limiting item IDs. It reports bags alone and all observed
storage separately. Unobserved storage, unsupported requirements and recipes
without fixed reagents can make a result unavailable. Partial storage gives a
lower bound for material capacity. Crafting eligibility remains unverified.

Each reagent also includes its observed stack count and up to 20 captured
locations, with storage, container, slot and stack quantity. Totals include
all observed stacks. These are locations in the saved export.

Pass `inventory_snapshot_id` to review a saved recipe against a separate
inventory capture from the same character and exact client/build. It must be
at least as recent as the recipe capture. Reagents come from the recipe
capture, while counts, locations and storage coverage come from the inventory
capture. The response includes both capture times. Missing storage stays unknown.

Supply up to seven `additional_recipes`, each with a saved `snapshot_id` and
`crafts`, to plan multiple distinct recipes against one inventory capture.
The primary capture provides the inventory if no separate capture is supplied.
`combined_materials` sums fixed demand and counts the shared stock once.
It is null if any recipe has unsupported requirements. The plan uses captured
stock and does not credit outputs from other planned crafts.

Use `find_snapshot_item` with a saved `snapshot_id` and a base `item_id` to
find captured copies in equipment, bags and banks without a simulator
catalogue. Use `offset` and `limit` (1–50, default 20) to page through matching
locations. Per-storage quantities cover every matching location. A null
quantity means the storage was unavailable in that capture.

Pass up to seven `additional_snapshot_ids` to check the same item across
explicitly selected captures from the same client and build. Each capture
retains its own quantities, coverage and pagination. The search applies the
same offset and limit to each capture. Shared bank observations stay separate.

Pass `currency_id` to `compare_snapshots` for a Forever currency's captured
before and after records, including its observed balance and cap fields.
The report gives the direction and amount of the balance change when both
records have the same account scope. An unseen ID or changed account scope
leaves the change unknown. Weekly earnings are shown as captured values.

Use `/aoexport wallet` to include the current character's carried money in
copper. Regular `/aoexport` omits it. The optional value is part of the saved
snapshot and follows its configured sync path. Read it with `inspect_snapshot`
and `section: "wallet"`. The capture uses Blizzard's `GetMoney` UI API.
Use a wallet-aware importer and reader before syncing these exports.
[Generated Forever UI documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/PlayerScriptDocumentation.lua)

Pass `wallet: true` to `compare_snapshots` to report the net change in copper
between two compatible wallet captures. A missing wallet leaves its value and
the net change unknown. The result reports balances and net change only.

Pass `faction_id` to `compare_snapshots` for captured reputation before and
after. The signed standing change is available when both records have standard
standing and the same account scope. Friendship, renown, paragon and unknown
progression types are shown as captured, with no calculated standing change.

Use `quest_id` or `achievement_id` with `compare_snapshots` to inspect a saved
quest or tracked Forever achievement across two captures. The response
compares observed readiness and failure flags for quests, and completion and
character credit for achievements when the account scope matches. Objective
and criteria details are shown as captured. Missing flags or records remain
unknown, so a quest disappearing from the log does not imply turn-in.

For tracked achievements with matching account scope, `criterion_comparison`
matches unique criterion IDs across captures and reports changed records.
Missing or duplicated IDs are excluded. A quantity change requires both
captured quantities and an unchanged required quantity.

- **Simulation.** [WoWSims Forever](https://github.com/wowsims/forever) revision `9dc32ac8ca0d8d1b76337861d38b93c0f884af11` built on Windows with Go 1.27.3, protoc 36.2, protoc-gen-go 1.36.10 and the upstream embedded database. A local SIML run used the upstream Survival Melee Hunter build over the Hunter defaults, including its explicit melee rotation, gear, raid buffs and one-target 180-second encounter. At seed 20261107 and 100 iterations, five independent CLI invocations agreed after sorting spell rows. Mean DPS was 770.047468 with standard deviation 36.709963. All inputs were public upstream fixtures. The MCP still needs a validated mapping from captured characters and owned items to simulator requests before it can rank legal sets.
- **Catalogue.** The [pinned WoWSims database](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/assets/database/db.json) parsed locally with 6,043 distinct item IDs. Only 3,945 have source records, and all carry phase value 1. SHA-256 `5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e`. Use item data for simulator compatibility and provenance-aware lookup. Use separately verified acquisition evidence such as [ForeverDiff](https://foreverdiff.gg/) and official unlocks for obtainable-gear claims. Blizzard lists the first three raids for 9 December. [Blizzard raid overview](https://worldofwarcraft.blizzard.com/en-us/news/24303009/world-of-warcraft-forever-raid-overview-locations-raid-sizes-and-attunement)
- **Optimisation.** Keep three candidate sets explicit: equipped gear, owned gear and obtainable upgrades. Filter by level, class, faction, weapon and off-hand rules, unique-equipped limits, profession and content availability. Evaluate full sets so set bonuses, caps, enchants, suffixes and procs can affect the result. Label unobserved inventory and unknown acquisition separately. Report the baseline, assumptions, coverage, tested candidate count and uncertainty with rankings. A bounded search result applies only to the evaluated candidates and encounter.

Run personal profiles locally by default. Keep provider revisions and data build claims separate from the character snapshot build. Preserve the distinction between a simulated recommendation and an in-game equipment change.
