# Forever

Owns Forever addon code, API adaptation, capability inventory and runtime qualification.

See the [interaction map](INTERACTIONS.md) for implemented coverage, planned analysis and actions that stay with the player.

## Find recipes supported by observed materials

To narrow discovery by saved recipe-view flags, add `recipe_view_filter`, for
example `{"learned":true,"can_skill_up":true,"minimum_skill_ups_reported":1}`.
Supply at least one non-null predicate. `learned` and `can_skill_up` are optional
booleans; the optional minimum is an unsigned 32-bit integer, including zero.
All requested predicates must match the same exact recipe ID in
`profession_recipe_view`, whose profession ID must match the bulk material
section. Missing rows, fields, sections or a different profession stay unknown.
A known predicate mismatch excludes the row even if another predicate is unknown.

The filter can combine with `material_filter` to discover reported skill-up
recipes supported by observed reagents. ID/status predicates run first, then
recipe-view predicates, then material support, before paging. View unknown and
nonmatching counts cover the first stage; material counters cover only rows
that passed the view filter. Returned `recipe_view_observation` fields retain
the captured flags and difficulty, with no recipe names. The coverage record
keeps `learning_verified` and `skill_gains_verified` false.

These are observations of the filtered local recipe view, not an estimate of
guaranteed skill gains or aggregate learning across conflicting sources. The
separate recipe review may still reject contradictory selected/tracked learning.
Recipes absent from the bulk material capture are not invented from view rows.
Omitting the view filter, or supplying null, preserves prior search behaviour.

`search_forever_recipes` accepts optional `material_filter`, for example
`{"scope":"bags","minimum_crafts":5}`. Scope is required and can instead be
`all_observed_storage` to combine observed bags, character bank and account bank
from the same snapshot. The minimum is a positive 32-bit integer. This finds
bulk-captured fixed recipes whose observed item counts support that many crafts;
it does not verify learning, cooldowns, conditions or actual crafting eligibility.
Each recipe is evaluated independently against the same stock, so matching rows
are alternatives, not a jointly affordable crafting batch.

Results keep numeric recipe ordering and existing pagination/ID/status filters.
`material_support` reports the observed `supported_crafts` bound. Explicitly
empty required reagent lists match any minimum, with `no_required_items: true`
and a null finite capacity. Unsupported/unreadable material rows or completely
missing selected storage are unknown and do not satisfy the filter. Unknown and
insufficient-observed counts cover rows passing the other filters, before
paging. Missing recipe observations yield null counts; observed-empty lists
yield zero. Inventory coverage reports missing sections and never verifies
complete ownership. Equipment, mail and merchant stock do not contribute.

Omitting the filter, or supplying null, preserves the original search result.
Use `review_forever_recipe` with the returned exact recipe ID for its separate
preparation and learning checks; an explicit learning conflict can still block
review even when observed materials support the request. No game action or
external lookup is performed.

## Execute a saved gear journal page offline

For execution across pages and a combined observed ranking, use
`python -m tools.run_forever_saved_journal --help`. It takes the same trusted
MCP/template/encounter/pinned-engine/private-checkpoint inputs, plus `--top`
(1–20, default ten) and `--max-pages` (1–1,000,000, default 64). Page size defaults
to eight windows. New-run, iteration and time budgets are shared across pages
and encounters. Repeat the same invocation to resume; it starts from the
beginning, revalidates saved work and reuses numeric checkpoints. It does not
accept a cursor that could silently skip earlier pages in the combined ranking.

`all_saved_resolved_results_complete` covers all resolved requests in the saved
journal only. `ranking_scope` says either `all_saved_resolved_requests` or
`completed_pages_only`. Ranked gear hashes include a saved window offset and
local review reference for inspection. Missing coordinates, unresolved items
and unknown ownership/model coverage remain limitations; all verified global
ranking, feasible-set and BiS flags remain false. This is a ranking of observed
weighted DPS point estimates, not a statistical optimality guarantee. See
[ADR 018](../../docs/ADR-018-saved-journal-ranking.md).

`python -m tools.run_forever_journal_page --help` exposes the same trusted MCP,
template, encounter, pinned engine and private simulation-journal arguments as
the saved-window encounter executor. It selects the next saved window starts
automatically, with `--page-limit 1..64` (default eight), instead of requiring
individual `--window-offset` arguments. Encounter lookup and weight arguments
may repeat; run, iteration and time budgets apply across the selected page and
its encounters. `--checkpoint-only` revalidates and reports saved results.

Start without `--after-window-offset`. If `journal_page.resume_same_page` is
true, repeat the same invocation to resume missing results. Otherwise pass the
returned `journal_page.resume_after_window_offset_decimal` as
`--after-window-offset` to select the next page. A null continuation with a
complete page means no later saved row was observed. Reaching the end of stored
rows does not establish complete search coverage. An empty page reports
`no_saved_windows_after_cursor` without running the MCP or simulator.

The cursor is a caller-selected position, not a receipt for previous pages.
Each comparison ranks only selected resolved work. Prior-page verification,
global ranking, feasible-set coverage and verified BiS remain false. Changed
selected rows or next-row observations reject the report; completed private
numeric checkpoints remain available. See [ADR 017](../../docs/ADR-017-saved-journal-page-execution.md).

Non-pet `trainer_services` can include optional `primary_profession_slots_full`
and per-lesson `is_profession`, reflecting the API's profession-acquisition
flag. With `inspect_snapshot.trainer_requirements_review: true`,
`trainer_requirements_review.profession_slot_checks` reports a slot blocker
only when the lesson flag and occupied-slot observation support it. Unknown
metadata remains null; an explicit non-profession lesson has no profession-
slot blocker. This is separate from price/budget projections and does not
verify eligibility. Changed lesson observations invalidate the trainer
section rather than mixing rows. No UI, training or profession changes are
performed. Update readers before syncing the added fields. See
[offline evidence](../../docs/trainer-profession-slots-verification-2026-11-08.md).

Saved `spellbook.entries` can now include optional `level_learned`, reported
by the public spell API for the captured spell ID. It can vary with level
linking and does not establish eligibility or current usability. A readable
stable zero remains zero; missing/restricted/invalid/changing data leaves
the field absent. Existing spellbook inspection filters return it normally.
With `compare_snapshots.spellbook: true`,
`spellbook_change.changes.observed_level_learned_changes` compares deduplicated
level observations only for shared exact base/reported spell-ID pairs.
Null in a level set means unknown; different duplicate values are retained.
This reports metadata changes rather than newly learned spells or level gains.
Update readers before syncing the new field. See
[offline evidence](../../docs/spell-learned-level-verification-2026-11-08.md).

`list_snapshots.spellbook_spell` finds saved rows matching a required
`spell_id` in either the base or reported ID field. Optional `passive`,
`off_spec`, `minimum_level_learned` and `maximum_level_learned` must all match
that same row. Unknown flags/levels never satisfy requested false/zero values.
For example, `{"spell_id":100,"passive":false,"maximum_level_learned":20}`
finds those recorded observations, not current spell usability or complete
learning. Results add `spellbook_spell_match`, with total matched slots and
up to 20 numerically ordered metadata rows; names are omitted. Character,
date, newest-capture and other feature filters still apply. Absence from the
saved view does not prove a spell unlearned. See
[offline search evidence](../../docs/spellbook-search-verification-2026-11-08.md).

The pinned simulator models level 60 only. Equipment reviews now expose
`captured_character_level` from saved stats, or null when unknown. The ranker
and direct adapter require an explicit integer 60 before constructing character-
bound simulation requests. A template field cannot change the engine's fixed
level. Other or unknown levels remain available through ranker `--plan-only`,
with `template_identity.level` reporting unsupported/unverified and
`simulator_character_level: 60`. Class/race checks remain in force.
Older exported reviews must be re-exported with an updated MCP reader; a saved
capture already containing level 60 does not need another game capture.
This prevents incorrect-level results; it does not add simulation support for
other levels. See [verification](../../docs/simulation-level-verification-2026-11-08.md).

Forever item observations can include optional `required_level`, read from the
captured item link after checking that its native base ID matches the observed
item ID. Missing APIs, unreadable data, unloaded details and errors leave it
unknown, never zero. Equipment and the shared bag/bank reader use the same
helper; it performs no equip or explicit data-load operation. Update readers
before using an exporter that writes this metadata.

Equipment reviews withhold simulator input when a proposed change has a known
native required-level shortfall against the captured player level. The
`native_required_level_shortfalls` check is separate from catalogue conflicts;
owned-gear target reviews and the ranker also report/skip these candidates.
Current same-slot equipment and the empty-selection baseline remain preserved.
Meeting the level requirement does not establish equip legality, proficiency,
specialisation compatibility or current usability. Existing catalogue checks
remain in effect. See [verification](../../docs/native-item-level-verification-2026-11-08.md).

Active combat/summoned pets are separate from the pet journal:
`section: "pet_care"` can now include optional `family_id` and `level` from
readable public unit metadata. Missing APIs or restricted/invalid values
leave those fields absent. Changing metadata makes the section unavailable.
`character_summary.active_pet_observation` reports presence plus nullable
family/level, or null for an unavailable section. Existing pet-care comparisons
include the two observed values when available, without inferring the same
individual pet or a level gain. No pet names, GUIDs, summon or training actions
are added. Update readers before importing the new fields. See
[offline evidence](../../docs/active-pet-details-verification-2026-11-08.md).

`pet_care.training_points` can include the public `total` and `used` counters
without a trainer window. Both must be readable and stable during capture;
missing or partial data stays absent. Summary `active_pet_observation` includes
the raw counters and `training_points_balance` as signed total minus used,
so deficits are preserved. Pet-care comparisons show the counters on each
side without deriving points gained or spent across potentially different
pets. This does not verify learned skills or lesson eligibility. Update
readers before using the new export field. See
[offline evidence](../../docs/pet-training-points-verification-2026-11-08.md).

`list_snapshots.active_pet` searches saved pet-care observations. For example,
`{"family_id":1,"minimum_level":60,"minimum_training_points_balance":10}`
requires all three recorded conditions. Use
`{"maximum_training_points_balance":-1}` for observed deficits, or
`{"present":false}` for an explicitly absent pet. Missing fields/sections
never match a requested threshold, even zero. At least one predicate is
required; absent-pet searches cannot also require pet details. Inclusive
level and balance bounds must be ordered, with balance bounds limited to
±4,294,967,295. Results include compact `active_pet_match` metadata.
Character, date, newest-capture and pagination filters compose normally.
These are saved observations, not the character currently in play or live
pet readiness. See [offline evidence](../../docs/active-pet-search-verification-2026-11-08.md).

Equipment reviews now include `captured_active_pet` with presence, family ID
and level, or null for an unavailable pet observation. The pinned engine also
fixes Hunter pets at level 60. If a Hunter template models a pet (nonzero
`petType` and positive `petUptime`), simulation requires a saved present pet
with integer level 60. Other or unknown pet levels remain available with
`--plan-only`. No-pet/zero-uptime templates and other classes are unaffected.
Rankings report `template_identity.hunter_pet`; direct adapter results report
`hunter_pet_assumptions` per review. Family, happiness and training remain
unverified even when level matches. Update the reader and re-export an
existing capture if it already contains the required pet metadata; otherwise
a new native capture is still needed later. See
[offline verification](../../docs/simulation-pet-level-verification-2026-11-08.md).

`inspect_snapshot` accepts `section: "pet_collection_view"` for species-level
observations while the pet journal is visible. The capture aggregates owned and
unowned rows by species ID, using public species names. Individual pet GUIDs,
custom names, levels, stats, favourites and the separate account-owned total
are omitted. `character_summary.pet_collection_view_counts` reports displayed
rows, observed species and owned/unowned row totals, or null when unavailable.
The journal remains filtered: absent species are unknown, and these counts do
not establish complete account ownership. The exporter does not open the
journal, change filters, rename, cage, release, summon or start pet battles.
See [verification](../../docs/pet-collection-view-verification-2026-11-08.md).

Set `pet_view_changes: true` in `compare_snapshots` to compare two saved
pet-journal views. Shared species report changed owned/unowned row counts;
one-sided IDs are listed as observed only before or after. Names do not
trigger changes. Missing either view returns null. Filter equality and
collection gains/losses remain unverified. Same-character/client and capture
ordering checks apply. See [comparison evidence](../../docs/pet-view-comparison-verification-2026-11-08.md).

Search saved species observations with `list_snapshots.pet_species`, for example
`{"family":"forever","pet_species":{"species_id":7,"minimum_owned_rows":1},"latest_per_character":true}`.
Only `species_id` is required. Optional `minimum_owned_rows` and
`minimum_unowned_rows` are inclusive and must match that same species. An
observed row is required even at zero thresholds. Bare species queries match
either ownership state in the filtered view; missing species never match.
Results include `pet_species_match` with the captured counts, public name and
partial coverage. They do not prove complete account ownership or absence.
Date, character, newest-capture and pagination filters still apply.

`inspect_snapshot` accepts `section: "mail_attachments"` for pending attachments
in the loaded inbox while the player's inbox panel is visible and idle. It
records loaded/total message counts and attachment positions, IDs and quantities
for readable items or currencies; unresolved slots remain `kind: "unknown"`.
It omits senders, subjects, text, invoices, money, COD and item modifiers.
The exporter does not refresh, open messages, retrieve, delete or send mail.
`character_summary.mail_attachment_counts` reports captured slot counts by kind,
or null when unavailable. Message/attachment positions identify only this
capture. Unloaded messages remain outside coverage. Pending attachments are
excluded from owned-item quantities, gear search and recipe material stock;
their presence does not establish ownership, retrieval eligibility or cost.
See [verification](../../docs/mail-attachment-verification-2026-11-08.md).

Set `mail_attachment_changes: true` in `compare_snapshots` to compare saved
known attachment totals by kind and ID. Shared observations report quantity
and attachment-slot count changes; one-sided IDs retain their known totals
without a zero or transfer assumption. Item and currency IDs are separate.
Message positions can reorder without a quantity change. Loaded/total message
counts and unresolved-slot counts keep partial coverage explicit. Missing
either section returns null; message identity, deliveries/retrievals and
inventory transfers remain unverified. No correspondence or mail actions
are added. See [offline comparison evidence](../../docs/mail-comparison-verification-2026-11-08.md).

`review_forever_recipe.pending_mail_material_observations` shows known pending
item attachments matching a fixed recipe's reagent IDs. It uses the selected
inventory capture, including for every recipe in a batch, and records that
capture's time and partial inbox coverage. Currency attachments and unrelated
items are excluded. Unseen reagent IDs stay separate from known quantities.
Missing mail or unsupported recipe requirements return null. The field has
`counts_toward_material_capacity: false`: stock, shortages, bank withdrawals,
craft capacity, dependency plans and merchant purchases retain their existing
inputs. Repeated batch observations do not allocate pending mail to recipes.
No retrieval eligibility is implied. See [offline evidence](../../docs/recipe-pending-mail-verification-2026-11-08.md).

Search pending observations with `list_snapshots.mail_attachment`, for example
`{"family":"forever","mail_attachment":{"kind":"item","id":720,"minimum_quantity":2},"latest_per_character":true}`.
`kind` is `item` or `currency`, with a positive ID in that namespace. The
optional minimum defaults to one and applies to summed known quantities in the
loaded inbox. An observed matching slot is required even when the minimum is
zero. Missing sections, absent IDs and unresolved slots never match. A failure
to match cannot establish absence from unloaded or unresolved attachments.
`mail_attachment_match` retains partial coverage and loaded/total counts,
the quantity across all matches and at most 20 numerically ordered locations,
with total matched count and truncation flagged. Date/character/newest filters
and pagination apply normally. These quantities remain pending mail, not stock.

Basic `professions` capture validates public names, ranks and skill-line IDs
and checks two consecutive reads for consistency. Missing or changing data
marks this section unavailable while retaining valid sections in the export.
This shared collector also applies to other client families. See the
[offline verification](../../docs/profession-capture-verification-2026-11-08.md).

Equipment reviews include `captured_professions` (null when unavailable).
They also include `captured_character_identity` with class and race, each null
when unavailable. Direct simulation now rejects a known class/race mismatch
before running any engine; gear ranking uses the same check. Legacy reviews
and unmapped identities remain explicitly unverified. Direct adapter results
include `character_identities` for each supplied review. This does not verify
talents, pets or current character activity. See the
[offline identity evidence](../../docs/simulation-character-identity-verification-2026-11-08.md).

The local simulator tools compare saved profession identities with the template's two
professions before simulating. A known mismatch stops the run; `--plan-only`
retains the draft and reports `template_identity.professions.status: "mismatch"`.
Missing observations or unmapped skill lines report `"unverified"` unless
another known captured profession already contradicts the template.
A match verifies identities only, not ranks, specialisations, recipes or
profession-dependent consumable eligibility. Direct adapter results include
`profession_identities`; rankings include `template_identity.professions`.
See [offline integration evidence](../../docs/simulation-professions-verification-2026-11-08.md).

`inspect_snapshot` accepts `section: "profession_recipe_view"` for the saved
filtered recipe list of a visible local crafting page. It includes the current
profession ID and recipe IDs, with names and learned flags where readable.
Linked, guild, NPC and inspection contexts are excluded. The exporter does not
open the page, change filters or choose recipes. Missing detail stays unknown;
absence from this filtered list does not mean a recipe is unlearned.
`character_summary.profession_recipe_view_counts` reports observed, learned,
unlearned and unknown-learning row counts, or null if the section is unavailable.
This is not a full learned-recipe catalogue or a crafting/material review for
every listed recipe. See [verification](../../docs/profession-recipe-view-verification-2026-11-08.md).

Search these saved observations with `list_snapshots.profession_recipe`, for
example `{"recipe_id":7,"profession_id":164,"learning":"learned"}`. Only
`recipe_id` is required; `learning` accepts `learned`, `unlearned` or `unknown`.
Even unknown requires the recipe ID to be present in the captured filtered list.
Missing and explicit null flags remain unknown, while unlearned requires an
explicit false flag. The result includes `profession_recipe_match` with the
coverage, profession/recipe IDs and recorded name/learning flag. Character,
date, newest-capture and pagination filters still apply. This search does not
establish current knowledge, full profession coverage or craft eligibility.

Set `profession_recipe_changes: true` in `compare_snapshots` to compare two
saved filtered recipe views. Shared recipe IDs expose before/after learning
flags only when profession IDs agree. Null retains unknown learning; missing
and explicit null flags are equivalent. Names are ignored. IDs present on only
one side are labelled observed-only, without inferring learned or lost recipes.
Different professions return `same_profession: false` and null derived changes;
a missing section returns null for the whole recipe-view comparison. Existing
character/client/build/time guards still apply. Changes describe observations,
not learning events or proof of identical filters.

Saved mount collections are available through `inspect_snapshot` with
`section: "mount_collection"`; `character_summary` reports the observed
collected count. This is source-tested read-only support, with native capture
still pending. Update both readers before using the new exporter.
See [mount collection verification](../../docs/mount-collection-verification-2026-11-08.md).

Add `mount_id` to that inspection to select one saved mount. An observed row
returns `observed_collected: true`; an absent ID returns null, not false.
`available` still describes whether the collection section was captured.
Use `mount_collection_changes: true` in `compare_snapshots` to compare compatible
captures. It lists IDs observed on only one side and changes in shared IDs'
spell, usability and favourite fields. Names are excluded. These observations
do not establish acquisition or loss, especially if catalogue coverage changed.

To search saved captures, pass `collected_mount` to `list_snapshots`, for example
`{"family":"forever","collected_mount":{"mount_id":2,"usable":true},"latest_per_character":true}`.
The mount ID is required; `usable` and `favorite` are optional recorded flags
and must match that same mount. Results include a compact `collected_mount_match`.
Missing IDs or collection sections never match, including searches for false
flags. Date, character and pagination filters still apply. This does not identify
the character currently in play or establish present mount usability.

`inspect_snapshot` also accepts `section: "toy_collection_view"`. The exporter
reads the filtered toy result list only while the toy panel is visible, across
the result pages without paging the UI. It preserves item IDs, names and
collected/favourite flags. `coverage: "filtered_journal"` means this is not a
full collection. `character_summary` reports `toy_rows_observed` and
`collected_toys_in_view`; zero describes an observed empty result, while null
means unavailable. The exporter never opens the panel, clears search text,
changes filters or uses toys. Update the importer and reader before installing
an exporter that includes this section; native qualification remains pending.

Add `toy_item_id` to toy-view inspection to select an item. `observed_in_view`
is true or false when the view was captured, and null when it was unavailable.
`collected_in_capture` is a recorded boolean only for an observed row; absent
items stay unknown. Set `toy_view_changes: true` in `compare_snapshots` to
compare overlapping item IDs' collected/favourite flags. One-sided IDs are
reported as observed only before/after, since journal filters may differ.
Names are ignored and missing views give a null comparison. These reports
do not establish toy acquisition, loss or full account coverage.

`inspect_snapshot` accepts `section: "saved_instances"` for saved dungeon/raid
metadata. Capture becomes available only after the addon observes an
`UPDATE_INSTANCE_INFO` event; entering the world again invalidates that marker.
The normal game UI can request the update when the player opens raid info.
The MCP does not request it. `update_age_seconds` records how old that update
was at capture. Names, difficulty, reported reset seconds, size and lock/extension
flags are retained; unique lockout IDs and group details are omitted.
`character_summary` reports observed entry and locked-entry counts, not present
entry eligibility. Timers are not advanced on read, and world-boss saves are not
part of this section. Reader/importer updates are required before installation;
native qualification remains pending.

Set `saved_instance_changes: true` in `compare_snapshots` to compare captured
reset timers and lock/extension flags. Matching uses case-insensitive public
name, difficulty, raid/dungeon kind and player capacity, not a unique lockout
ID. Duplicate descriptions on either side are reported as ambiguous with no
deltas. One-sided descriptions are observations only; they do not prove a reset
or new lockout. Missing cache sections remain unknown, and cache ages describe
the capture time. Ordinary character/client/build/time comparison guards apply.

`inspect_snapshot` also accepts `section: "saved_world_bosses"`. It records
world-boss names and reset timers separately from dungeon/raid instances after
the same passive instance-info update. Unique identifiers are omitted.
`character_summary.saved_world_bosses_observed` distinguishes a captured empty
list (zero) from an unavailable cache (null). These observations do not establish
present loot eligibility; timers and update age describe the capture time.

Use `list_snapshots.saved_world_boss` with a required exact case-insensitive
`name` and optional `maximum_reset_seconds` and `maximum_update_age_seconds`.
The timer ceiling matches only nonnegative timers. All limits describe captured
values, not a countdown to today. Missing sections and names never match.
Results include `saved_world_boss_match` with cache age, matched count and up to
20 rows sorted by captured slot; `truncated` marks further matches. Combine
with character/time filters and `latest_per_character` to search each character's
newest valid capture within the requested time window. This does not identify
the character currently in play or prove that any boss is available for loot.

For saved-instance searches, use `list_snapshots.saved_instance`, for example
`{"name":"Molten Core","locked":true,"maximum_reset_seconds":86400}` with
`family: "forever"`. Names match exactly without case sensitivity. Optional
`difficulty_id`, `extended` and `is_raid` flags apply to the same row. Negative
reset timers are excluded when a maximum is supplied.
`maximum_update_age_seconds` limits the cache's age at capture, not its age now.
Results include a count and up to 20 matching rows, with `truncated` when more
match. Character, capture-time and newest-per-character filters still apply.
No row or unavailable cache is never interpreted as an unsaved character.

## Launch checkpoint, 7 November 2026

The local `_forever_` installation identifies itself as product `wowf`, build `1.60.1.70862`. The build probe does not establish the interface number. Interface `16001` is the package candidate based on the prior fixture and existing Forever addon TOCs. The packaging command writes the requested variant into the generated adapter, so the launch package uses `live` even though the checked-in early adapter says `beta`.

The `0.20.0-dev` Forever package passed byte and manifest verification, a packaged Lua 5.1 export/import round trip and a changed-interface rejection. Source base `4118e51`, SHA-256 `1655f34082acab17adff1166def7315f5fdbcee418492af212f76e3b0eb558f8`. Azeroth Operator was absent from the Forever installation. No installation, game input or native qualification was performed.

## In-game coverage

### Capture-only installation checkpoint

Forever `0.27.0-dev` contains only the adapter, collectors, export core and TOC. The ZIP SHA-256 is `e6190ec5842b0d273a029b7e87bb57ab5622890d9dff78104f40a2f1bd87a803`. The exact package passed a Lua export/import round trip, interface mismatch refusal and absence of the Apply handler. Retail and Classic package entries were verified byte-identical against the previous packager.

Package verification now runs the detailed Forever fixture against the actual staged Lua files through the registered `/aoexport` command. This covers banks, selected recipes, faction standings, currencies, tracked achievements and quest progress, including malformed and unavailable cases. The same ZIP was first-installed and hash-verified on a synthetic Windows client. Retail and Classic fixture packages passed the unchanged verification path. These are offline checks.

The Windows release companion imported the complete synthetic Forever fixture. Its executable SHA-256 `aae7f17c0762d54a42ff76cc92aad4dbb5f27d7b00d84577931d92db6ba86550` was verified and selected in the existing sync configuration. The previous configuration is backed up in `local-data/sync/config-before-forever-capture.json`. The scheduled runner matches its source and ran at 16:15:41 SAST on 7 November 2026 with result 0. Forever returned `waiting_for_export` and Retail returned `unchanged` from its cached acknowledgement. The task reported no live game access.

The staged 0.28.0-dev package adds fixed-material review for selected enchant recipes using the existing snapshot format. SHA-256 `164c6442250d4f1a85c656c231de6e89bfe0c05aad91528ac349f3b2aaa9539a`. Packaged Lua round trip, changed-interface refusal and the synthetic Windows installer test passed offline.

The staged 0.29.0-dev candidate adds the optional weekly currency earning
flag. The archive is in `dist/forever-weekly-currency` with SHA-256
`ce47230a500f3add4f9142eb5e82537e37f34c2755f4e05a27f95cf6282d3ad6`.
The packaged Lua round trip passed, including false, missing and simulated
restricted flags. Upgrade the Rust reader and importer before using this
candidate with hosted sync.

The staged 0.30.0-dev candidate adds optional renown observations. The
archive is in `dist/forever-renown` with SHA-256
`cc97e809be65742533bfe6bc8073cbb695601efe868c50d28380f4b82d3fc2e6`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with these fields.

The staged 0.31.0-dev candidate adds optional friendship reputation
observations. The archive is in `dist/forever-friendship` with SHA-256
`2860263ba501b4a4319e08a524da1054abde6471c16d19cf5c8da36eb3fd8b90`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with these fields.

The staged 0.32.0-dev candidate adds optional paragon reputation observations.
The archive is in `dist/forever-paragon` with SHA-256
`3cd3c3863880c866b039884d8b70af3e5d83a5571eacfdda7bf873ea82982dff`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with these fields.

The staged 0.33.0-dev candidate adds optional recipe training indicators.
The archive is in `dist/forever-recipe-training` with SHA-256
`830decf33ffde52392a33f834ba212131e7b5f0eee2a820bb282ac376841dd24`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with these fields.

The staged 0.34.0-dev candidate adds optional base item output ranges.
The archive is in `dist/forever-recipe-yield` with SHA-256
`97fbbaa3368091213ce859bbd02194c01b75600c76901f4b37296b0da97c67d5`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with this field.

The staged 0.35.0-dev candidate adds optional recipe cooldown and charge
observations. The archive is in `dist/forever-recipe-cooldown` with SHA-256
`0dab1f3299528a42dda5361e4591cb03a5dc23dbbe75155e356d85792d5c4d00`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with this field.

The staged 0.36.0-dev candidate adds optional recipe conditions.
The archive is in `dist/forever-craft-conditions` with SHA-256
`37f18cefc90fd50c76d394b1d37a2ff91e7d5056570269a0d9409f084edd75ab`.
The packaged Lua round trip passed. Update the Rust reader and importer before
syncing captures with this field.

The staged 0.37.0-dev candidate adds tracked recipe and recraft observations.
The archive is in `dist/forever-tracked-recipes` with SHA-256
`702d9d417e2530da4cc24cbfadec5860e943ede761bc1977b99d333986989fe7`.
The packaged Lua round trip passed. Update the reader and importer before
syncing captures with this section.

The staged `0.38.0-dev` quest review candidate is in
`dist/forever-quest-review`, built from `261873b` for interface `16001`.
The ZIP SHA-256 is
`57eb2a06ca64c122a0faf4ab4a00bc57e36ed2fbe66ca4c75d8e9af548f826e5`.
The packaged Lua fixture passed export/import and rejected a changed
interface. The same ZIP installed on a synthetic Windows client with all
four hashes matching and refused to overwrite the existing directory.

The matching staged Windows release companion is
`dist/forever-quest-review/azeroth-operator.exe`, SHA-256
`5cba3812f0e8352e67d218e3884d7068cbda0f6eb067ccc2864b56a803467b06`.
It accepted synthetic quest data via `import-stdin` and served the timer,
difficulty and waypoint-filtered checklist through MCP stdio. Update the
reader and importer before syncing these fields. Native capture is pending.

The staged `0.39.0-dev` candidate is in `dist/forever-progression`, built
from `289aa0e` for interface `16001`. The addon ZIP SHA-256 is
`3ab8e4227bfe659f8bf179f1542b935fb159df4082256455ff28667e033d22ba`.
The packaged Lua fixture passed export/import and changed-interface refusal.
A synthetic Windows install matched all four manifest hashes and refused
to overwrite the existing addon directory.

The matching Windows release companion is `dist/forever-progression/azeroth-operator.exe`,
SHA-256 `e47cd8275af5105d7b414e735a3c92fde5d7383688434e88e5e5077061c079c9`.
It imported synthetic data with the experience section and served XP
progress, the quest XP projection, reward spell filtering and recipe
observations over MCP stdio. Update the reader and importer before syncing
this candidate. Native capture is pending.

The staged `0.40.0-dev` candidate is in `dist/forever-action-review`, built
from `0adbdc3` for interface `16001`. Its ZIP SHA-256 is
`b498aa3de88f6859df5267f0b8345109a26205136cf9d5a54b92554d3b19efba`.
The exact ZIP passed the packaged Lua fixture, changed-interface refusal
and synthetic Windows installation with all four files verified. The
installer preserved the existing directory on a second invocation.

The matching Windows release companion is `dist/forever-action-review/azeroth-operator.exe`,
SHA-256 `b4191c95c27ee3dee1ddcbe4aee8a425d1fa8f4c277ff9bd145a8b1685db5f55`.
It imported synthetic spellbook and action bar captures and served name
and hotkey searches, spellbook matches and hotkey changes over MCP stdio.
Update the reader and importer before syncing this candidate. Native capture
remains pending.

The staged `0.41.0-dev` candidate is in `dist/forever-travel`, built
from `04a3c5f` for interface `16001`. Its ZIP SHA-256 is
`78d6ed6974ff50019a0a90a21e47d87c94896cb34925f95b27187e4007a220ae`.
The archive passed packaged Lua 5.1 verification, changed-interface
refusal and synthetic Windows installation with all four files verified.
The installer refused to overwrite the synthetic install.

The matching Windows release companion is `dist/forever-travel/azeroth-operator.exe`,
SHA-256 `54dcc6c23430eba92585a29fb8e04f8e5ffb5f05db7496184d563e5df5e3cfd5`.
The installed fixture's `/aoexport wallet` output passed through
`import-stdin` into a temporary owner store. MCP stdio returned the
Hearthstone timer, repair and flight shortfalls, general bag space,
instance lock warning details and map-ID filtering. Filtered listings
omitted warning details. Native capture is pending.

After closing Forever, first-install the reviewed capture package from the repository root:

```powershell
$manifest = (Resolve-Path 'dist/forever-travel/AzerothOperator-0.41.0-dev-forever-live.manifest.json').Path
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

Set the same environment variable in the local stdio MCP client's server configuration. The optional `lookup_forever_catalogue` tool uses that file for catalogue records. For example, use `{"section":"items","key":"744"}` for an item, `{"section":"npcs","key":"ragna"}` for a name search, or `{"section":"encounters","key":"Default/Raid Target"}` for a simulator encounter preset. Omit all filters for a section count. Lookups return at most 20 matches. If `has_more` is true, keep the same filters and advance `offset` by 20.

The sync command uses curl for a bounded HTTPS download, checks the SHA-256 pinned in `catalogue.lock.json` and replaces the destination atomically after verification. A matching cache skips the network. Upgrading data requires reviewing and updating the revision and hash in the pin, rebuilding the companion, and rerunning sync. The catalogue includes simulator reference zones, NPCs and factions. For a wider world mirror, qualify an exportable Forever source independently for coverage, build identity and redistribution terms before adding it.

For crafting references, pass `{"section":"items","recipe_spell_id":2675}` to find catalogue items associated with that recipe. The `enchants` section supports the same filter for enchant references. A supplied `key` further narrows the matches. These associations come from the pinned simulator catalogue and can be incomplete or list several variants. Use the saved recipe observation for reagent requirements.

Use `simulator_slot` and `minimum_item_level` to narrow gear searches.
Use `crafted_profession_id` in item or enchant lookups to find outputs
for a pinned simulator profession, such as Blacksmithing (2).
If `recipe_spell_id` is also supplied, the same crafting source must
match both. These records identify catalogue outputs, and recipe
availability still depends on separate evidence.
With an item lookup and `acquisition_snapshot_id`, enable
`only_observed_learned_recipe` to restrict results to the capture's
positive selected-recipe, filtered profession-list or reported known-profession-spell observations,
joined by exact recipe spell ID. Conflicting known learning flags or a selected
output that contradicts the catalogue prevent a positive match. Unlisted or
unknown rows do not prove a recipe is unlearned. Tracking alone is insufficient,
and an empty result describes that capture.
The same filter works for enchants. It matches the crafting spell ID
and reports the capture ID under `recipe_observation`. When both sides
record an output item ID, those IDs must agree.
Crafted acquisition sources include `recipe_learning` with independent observations,
their conflict flag and combined nullable learning result. The existing source
`learned`, material capacity, output yield and cooldown fields continue to use
only the compatible selected schematic; a learned list row supplies none of
those crafting details. These searches do not establish current craftability.
[Offline evidence](../../docs/catalogue-recipe-learning-verification-2026-11-08.md).

The separate `profession_spells` section records the public API's known spell
IDs for the visible local profession, without changing filters. Its summary
shows the reported count and first 50 numeric IDs. `recipe_learning` includes
`profession_spell_reported`; an exact crafted-source ID is positive evidence,
but conflicts with false selected/list learning remain unknown. Unreported IDs
are not inferred unlearned, and this is not a complete recipe enumeration.
[Offline evidence](../../docs/profession-spells-verification-2026-11-08.md).
For enchant lookups, supply `enchant_target_item_id` to keep effects that
fit the specified base item under the pinned simulator rules. It combines
with crafting filters and does not require a character capture.
The `enchant_target_preview` reports regular stats before and after each
returned enchant at default scaling. Add `enchant_target_random_suffix_id`
to include a listed suffix in the base. These totals exclude procs and
pseudo-stats. Missing stat evidence yields null.
Use `enchant_application_item_id` to search by the item that applies the
effect, such as a shield spike. Combine it with `enchant_target_item_id`
to check a specific pairing under the simulator rules.
For `random_suffixes`, use `suffix_target_item_id` to list the catalogue
suffixes referenced by a base item. For `items`, use
`item_random_suffix_id` to find base items that list a specific suffix.
These are catalogue possibilities. Saved links show which variant was observed.
Targeted suffix lookups include `suffix_target_preview` for the returned
page, with default-scaling regular stat changes, pseudo-stats and any
recorded replacement weapon damage. Missing scaling evidence yields null.
Each variant also includes its resulting regular stats before enchants,
combining the base item's regular stats with the suffix allocation.
Use `minimum_required_level` and `maximum_required_level` to find items
whose recorded character level requirement falls in an inclusive range.
These filters exclude items with no recorded requirement. Equip
eligibility and acquisition depend on other evidence.
The minimum uses the item level from the catalogue's default scaling record.
Both filters combine with source and ownership filters before pagination. Slot
matching uses the pinned simulator's item placement rules. This gives
candidate gear for the slot, with character eligibility and complete-set
legality evaluated separately. With
`acquisition_snapshot_id` and `simulator_slot`, `equipped_slot_comparison`
shows each returned item’s default catalogue level and the difference from
the saved item in that slot. If the equipped item or its catalogue level
is unavailable, the difference is null. The comparison also reports nonzero `default_regular_stat_delta` values by
simulator stat ID. A null delta map means the default scaling record was
unavailable. Stat gains are positive and losses are negative. These are
regular base stats only, excluding weapon damage, pseudo-stats, procs and
modifiers. The comparison uses base IDs and
does not account for modifiers or measure damage. It also shows
`catalogue_effect_buff_changes` for buffs added or removed from the two
catalogue records, including nested stacking aura IDs. An uncatalogued
baseline gives null. An item with no recorded effects contributes an empty
set, which describes catalogue coverage only.
`single_slot_set_projection` reports captured and projected piece counts
for sets affected by a one-slot replacement. Uncatalogued equipped items
make counts lower bounds, and missing equipment makes the projection null.
The projection does not infer set bonus activation or other slot changes.
Set `preserve_equipped_set: true` to exclude candidates that would remove
the saved slot's recorded set piece. It requires a captured slot and, if
occupied, a known catalogue item. A saved empty slot or an item without a
recorded set applies no set restriction. Set
`minimum_item_level_gain` to a positive number to keep only candidates
meeting that gain before pagination. This filter requires a known equipped
level in the requested slot. Use `minimum_stat_gain` with `stat_id` and
positive `minimum_gain` to filter by one regular base stat, such as Strength
(ID 0). This requires a known default scaling record for the equipped item.
A stat omitted from a present stat map counts as zero. Use
`order_by: "item_level_desc"` or `"stat_gain_desc"` to order matches before
pagination. Stat ordering uses the stat in `minimum_stat_gain`. Equal values
retain catalogue order. Missing item levels sort last. Use
`order_by: "weapon_dps_desc"` to order by recorded default weapon DPS,
with missing values last. Saved slot comparisons show the weapon DPS
values and their difference where both are known. These use the pinned
unrounded weapon DPS field and describe weapon damage per second only.
Use `minimum_weapon_dps`, `minimum_weapon_speed` and
`maximum_weapon_speed` for inclusive limits. Speed is seconds per swing.
A record missing a requested value is excluded.

With a Forever `acquisition_snapshot_id`, set
`exclude_observed_requirement_conflicts: true` to keep equippable catalogue
records and remove candidates with a known level, class, proficiency,
faction or profession conflict. Missing observations leave those checks
undecided. The filter evaluates individual catalogue items and does not
verify complete-set legality or live equip eligibility. With snapshot context,
returned gear can include reasons in `observed_requirement_conflicts_by_item_id`.
An omitted ID means no conflict was found in the available observations.

Use `item_effect_buff_id` to find items with a recorded buff spell ID, including a nested stacking aura ID.
`item_effect_trigger` accepts `on_use`, `proc`, `stack_proc` or
`stacking_aura`. When both filters are supplied they must match the same
effect. These are catalogue labels and do not predict in-game activation.

For raid supplies, use `section: "consumables"` with `acquisition_snapshot_id`
and a name, `item_ids` or `observed_ownership`. The result reports saved
counts for the returned potion, food and other consumable IDs. Item-source
and equipment filters remain limited to the items section.

Filtered consumable responses include `recorded_shared_cooldowns`.
Each group covers at least two returned items with the same positive on-use
cooldown category and lists the positive recorded durations per item in
milliseconds. To compare a specific pair, look up both IDs in one request.
Groups cover the returned page and describe catalogue data. Live cooldowns
and categories without a recorded positive duration are outside this view.

Use `consumable_group_key` with the exact `groupKey` from a catalogue
record to find alternatives in the same simulator buff group. It combines
with saved ownership filters. Filtered results and restock plans list
`simulator_exclusive_buff_groups` when two or more items on the returned
page share a recorded group. The groups describe the pinned simulator's
buff rules. Live stacking and ungrouped effects require separate evidence.
The local simulation and ranking commands check template selections against
the same groups when `AZEROTH_FOREVER_CATALOGUE` is configured. They reject
two distinct selected item IDs in one group before launching the simulator.

For a restock plan, supply `consumable_targets` instead of lookup filters,
for example `[{"item_id":118,"quantity":10}]` with the same section and
snapshot. Use up to 20 distinct IDs. The plan compares each target against
saved bag counts, suggests manual withdrawals from the character bank first
and reports any remaining shortage against observed stock. A positive
shortage is confirmed only when bags and both banks were captured. Without
bag coverage, withdrawal suggestions are unknown. Unmatched catalogue IDs
are listed separately in the plan with `catalogue_match: false`.

To find a saved schematic for an item you need, call `list_snapshots`
with `family: "forever"`, `recipe_output_item_id`, and the crafter's
`character` name and realm. The list returns captures where the selected
recipe recorded that output, newest first. Its recipe metadata includes the
recipe ID and the observed learned flag. Pass a matching capture ID to
`review_forever_recipe` with `target_output_quantity` to plan ingredients
when the schematic captured a supported yield. Recipes without captured
output IDs are outside this search.

Add `restock_recipe_snapshot_ids` to a consumable restock request to
calculate crafts for confirmed shortages after observed bank stock. Supply
up to eight compatible captures with learned fixed-material recipes and
known yields, one per shortage. Recipe captures must be for the same
character and client build and be at least as old as the inventory.
The combined ingredient list counts shared stock once. The request rejects
a recipe that would consume an item reserved as another restock target.
Crafting and bank withdrawals remain player actions.

Use `item_ids` in place of `key` to look up up to 100 distinct positive
item IDs. Results keep catalogue order and existing pagination. Source
filters and optional snapshot context apply to the matching records. With
`acquisition_snapshot_id`, the response includes `owned_quantities_by_item_id`
for returned item IDs. Counts group by base item ID across captured storage.
An observed zero is distinct from an uncaptured storage section, which gives
null. Slot locations and item links stay in the inventory tools.

With `acquisition_snapshot_id`, use `observed_ownership` to filter by base
item ID before pagination. `owned` requires a captured copy. `absent`
requires all four storage sections and no captured copy. `unknown` means no
copy was captured and at least one storage section was unavailable.

Use `{"section":"items","source_kind":"pvp"}` to browse PvP-tagged items. The other source categories are `crafted`, `drop`, `quest`, `vendor` and `reputation`. Filters combine at item level, so an item can match through multiple recorded sources. The catalogue PvP tag supplies no rank, cost or season data.

Use `drop_instance_kind: "dungeon"` or `"raid"` to filter recorded
drop categories. When supplied with `drop_npc_id` or `drop_zone_id`, the
category and IDs must match the same source. Sources with unknown categories
are excluded from this filter. Use `drop_label` for an exact ASCII
case-insensitive match on recorded source labels such as `Trash`, `World Drop`
or `Relic Coffer`. Any other drop filters must match that same source record.

For recorded loot sources, pass `{"section":"items","drop_npc_id":644}`. Resolve NPC names through the `npcs` section first. Results include only items with an explicit matching NPC drop in the pinned catalogue. For zone loot, use `drop_zone_id` to include both NPC and unattributed drops recorded for that zone. When both filters are supplied, they must match the same drop source. Missing entries provide no evidence about live loot availability.

Add `include_source_references: true` to a filtered item lookup to resolve
recorded drop and vendor NPCs, zones and reputation factions against the same
pinned catalogue. Each reference identifies the item and one-based position
of the source in its original record. A null name means the source ID has
no name in the catalogue reference table. Live acquisition remains unverified.

Pass `acquisition_snapshot_id` to a filtered item lookup to include saved
quest readiness and reputation observations for recorded source IDs. The
snapshot must be Forever and come from the configured owner store. A quest
absent from the capture has unknown progress. The result reports captured
standing and source requirements separately, leaving acquisition unverified.

For a crafted source, the same lookup checks the saved selected recipe's ID.
It also reports whether the source recipe ID was tracked as a craft or recraft
when the matching lists were captured. Missing lists give null status. Tracking
is an observation about the recipe ID, independent of output matching or learning.
When the recipe matches and any observed output agrees, it reports learned
status and material capacity from that capture. An absent output ID leaves
output matching unknown. An explicit mismatch suppresses learned status and
material capacity for that item. The matching source also reports the saved
one-craft base output range, cooldown and number of unmet conditions when
observed. Condition names stay in the recipe review. Cooldowns and conditions
reflect the capture time. Craftability remains unverified.

For vendor references, pass `{"section":"items","vendor_npc_id":1297}` or filter by `vendor_zone_id`. You can also filter by `vendor_faction: "alliance"` or `"horde"`. The NPC, zone and faction filters must match the same sale record. Untagged sales are excluded when the faction filter is supplied. Matching sale records can include the vendor name, zone and faction. With `acquisition_snapshot_id`, vendor and reputation sources can compare their recorded faction tag with the character's saved faction. An absent or unknown tag or character faction gives null. The faction match describes the catalogue tag only. Current stock, prices and purchase eligibility require separate evidence.

For reputation rewards, pass `{"section":"items","reputation_faction_id":509}` using a reputation faction ID from the saved standings or the catalogue. Use `reputation_required_tier` for an exact required rank such as `honored`, and `reputation_source_faction` for an `alliance` or `horde` tag. These selectors and the reputation faction ID must match the same source. Unknown tiers and untagged records are excluded when their corresponding filter is supplied. With `acquisition_snapshot_id`, `observed_reputation_requirement_met` compares the saved standard faction reaction to the recorded tier. Missing observations, unknown tiers and special progression give null. This field evaluates the reputation tier only. Character eligibility requires separate verification.

For quest rewards, pass `{"section":"items","reward_quest_id":53}` using the quest ID from a saved quest observation or other reference. Results cover reward associations recorded in the pinned catalogue. With `acquisition_snapshot_id`, matching quest IDs also show captured unfinished objectives and remaining counts when available. Turn-in readiness and failure remain separate observations. Character eligibility and live availability require separate evidence.

With a saved gossip capture, set `gossip_quest_list` to `available` or `active`
and supply `acquisition_snapshot_id` to discover reward references for its observed
quests. Optional `gossip_quest_is_complete` requires a matching known flag; unknown
does not match false. An additional `reward_quest_id` must match that same quest
source. `gossip_quest_filter.list_observed` distinguishes unavailable and empty
lists. Acquisition annotations retain dialogue observations separately from the
quest log. These are historical catalogue associations, not native reward promises
or acceptance/turn-in eligibility.
[Offline evidence](../../docs/gossip-reward-lookup-verification-2026-11-08.md).

For a public set reference, pass `{"section":"items","set_id":495}`. For a character comparison, call `review_forever_owned_gear` with the same `set_id` and a saved snapshot ID. Its `set_observation` lists each catalogue piece, whether its base ID was observed, and the number of occupied locations by storage. Duplicate copies count once toward distinct pieces. The summary spans the whole snapshot regardless of page, item ID or equipment category. Use storage coverage to see which sections contributed. An unseen piece may be in uncaptured storage. Set bonuses, acquisition and pieces in uncaptured storage need separate evidence.

Verified on 7 November 2026: the actual sync command returned `synced` then `unchanged`; a bad download preserved the destination. The Rust stdio MCP returned exact pinned records across all nine lookup sections. The unconfigured server retained 14 tools. Caller and failure review caught the lack of a total HTTP deadline in the initial implementation. The revised command has a 30-second transfer deadline and a 35-second process timeout. Jev advisory triage classified the finding as material (model `jev-1.14.0`, request `b3f8fe8728dbc6193bf06ddff54922a80d21c1f0980322deea9af094d055e3fb`, 422 input/46 output tokens, 0.304 seconds).

### Simulation and optimisation

Use `--healing-metrics` with the standalone simulator or `--rank-by hps`
with the owned gear ranker. The fixed comparison reports healing and
applied shielding per second as counted by the pinned engine, its standard
deviation, and average seconds out of mana. The single-player template
supplies the rotation and healing targets, so the result covers that
scenario. Bulk and weighted modes still use DPS. Add `--paired` to report
the standard error of the HPS difference against the equipped baseline.

Use `--rank-by oom` for lowest average time out of mana. Both healer
rankings list `healing_tradeoff_reviews`, the review IDs for which no other
tested loadout has at least as much healing and at most as much time out of
mana while improving either metric. Equal scores are kept. The list
includes equipped gear and uses point estimates for the specified rotation.

To heal a simulated ally, set `raid.targetDummies` to an integer from 1 to
24 in the template. Each active party has five slots, so a one-party
template has room for four dummies. Include additional empty party
objects when needed and keep those parties active. For a first-slot player and one dummy, an APL friendly
cast can target `{"type":"Player","index":1}`. Assign that dummy as the
tank in `raid.tanks` to give the encounter a separate incoming-damage
target. The ranking changes only the real template player's equipment and
reports that player's healing. Dummies model synthetic targets.

Use `--tank-metrics` with the standalone simulator to report mean damage
taken per second (DTPS), threat per second (TPS), their standard deviations
and the fraction of iterations with a simulated death. For owned gear, use
`--rank-by dtps` for lowest mean DTPS or `--rank-by tps` for highest mean TPS. In both
commands the template must assign its first player as the sole tank and at
least one encounter target must point to that tank. Comparisons also report
changes from equipped gear. Bulk shortlists and weighted encounter
rankings remain DPS modes. Paired runs include DTPS and TPS standard
errors against the baseline. Death
frequency depends on the encounter and template's healing assumptions.
Use `--rank-by death` to order the fixed comparison by fewest simulated
deaths, with ties in input order. All tank rankings list
`tank_tradeoff_reviews`, the review IDs for which no other tested row
has at least as much threat, at most as much damage taken, and at most as
many simulated deaths while improving at least one metric. Equal scores
are kept. The list compares point estimates and includes equipped gear.

For `Default/Custom Boss`, add `custom_boss` to the `simulation` object
with any combination of `thrash`, `cleave` and `magic_spell`. Thrash takes `extra_attacks`,
`proc_chance_percent` and `cooldown_seconds`. Cleave takes
`cooldown_seconds` and `bonus_damage_percent`. A 50% cleave bonus means
150% of the rolled weapon damage before mitigation. Cooldowns use whole
seconds. Configure a tank in the simulator template to measure incoming
damage from these attacks. [Pinned custom boss engine](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/encounters/custom_boss_ai.go)

Magic spells take `school`, `cast_seconds`, `cooldown_seconds`,
`minimum_damage` and `maximum_damage`. Schools are `arcane`, `fire`,
`frost`, `holy`, `nature` and `shadow`. A zero-second cast is instant.
The damage range is the pre-mitigation roll and is bounded to 0–1,000,000.

For `Default/Dynamic Adds`, add
`"dynamic_adds":{"spawn_delay_seconds":10,"lifetime_seconds":20,"respawn_seconds":10}`
to the encounter's `simulation` object. A zero spawn delay starts the first
wave immediately. A zero respawn delay starts the next wave immediately. Wave lifetime
accepts 1–3,600 seconds and both delays accept 0–3,600 seconds. The preset's
add count stays fixed. Optional fields include `kill_share_percent` (0–100),
`apply_raid_debuffs`, and `debuff_delay_seconds` and
`debuff_jitter_seconds` (0–3,600). Omitted fields use preset defaults.
Delay and jitter require raid debuffs to be enabled. These controls affect
the raid's configured debuffs on adds. Debuffs cast by the simulated player
still follow the rotation. [Pinned add-wave engine](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/encounters/dynamic_adds_ai.go#L160-L267)

For `Default/Movement`, add
`"movement":{"interval_seconds":10,"distance_yards":5}` to the encounter's
`simulation` object. Intervals accept 1–3,600 seconds and distances accept
0–100 yards. The override changes only those two target inputs. The pinned
engine defers movement until a non-movable cast ends, so the export offers
no reaction-time override. [Pinned movement engine](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/encounters/movement_ai.go#L78-L137)

For a bounded shortlist check, add `--audit-shortlist` to a ranker command
using the bulk adapter. The ranker runs every distinct gear input at the
encounter export's fixed iteration count, then compares its top candidates
with the bulk shortlist. It reports missed review IDs, fixed top selections
and both wall times per encounter. Equal point estimates use input order
as the tie-breaker. The fixed run uses the existing comparison budgets.
The runs may reach different precision, so a disagreement calls for closer
comparison at higher precision.

For repeated-seed qualification of an already exported request pool, use
`python -m tools.qualify_forever_shortlist --pool <pool.json> --engine <fixed.exe> --engine-sha256 <hash> --bulk-engine <bulk.exe> --bulk-engine-sha256 <hash> --seed 100000000 --seed 200000000 --seed 300000000 --top 5`.
The input is an object with a `requests` array containing the baseline first,
followed by explicit gear requests for one encounter and character context.
Only equipment may differ. Each pass replaces every request's seed with the
specified seed. The utility runs fresh comparisons without persistent caching,
collapses gear aliases to their first review and reports missed fixed-run top candidates,
stage telemetry and elapsed times, and includes an input digest rather than
the input contents. It permits one to three distinct seeds, 2–129 requests,
200–5,000 fixed iterations per request and the existing 100,000-iteration
fixed-comparison budget. Engine deadlines still apply to each comparison.
Duplicate baseline requests are not contenders. Empty contender lists do not
establish anything about ranking quality. Agreement measures point-estimate
shortlist recall on those inputs, not equip legality, global optimality or a
confidence guarantee; seed spacing alone does not prove statistical independence.

An offline two-slot audit used the public Hunter fixture with Icemail
Jerkin and Brawler Gloves as alternatives. It compared four distinct gear
sets including the baseline at 5,000 fixed iterations each. The real MCP
exported `Default/Raid Target`, `Default/Movement` and
`Default/Dynamic Adds` with a 90-second duration, five seconds of variation
and seeds 20261107 and 20261108. All six runs agreed on the ordered top two.
The bulk engine used one pass at 10,000 iterations in these cases, so the
check compares encounter handling and ranks without exercising culling.

A larger two-slot audit used the same baseline, eight chest alternatives
and seven hand alternatives. Each list took the first catalogue rows in
file order for level 60 or below, cloth, leather or mail, with no class,
faction, profession, recorded effect, uniqueness or limit-category filter
match. It excluded the equipped item in each slot. Both 90-second profiles
used five seconds of variation. Fixed mode ran all 72 sets, including the
baseline, at 1,000 iterations each. The bulk engine screened 71 candidates
at 1,000 iterations and advanced five to 10,000 in both cases.

| Preset | Seed | Fixed wall time | Bulk wall time | Fixed top five missed |
| --- | ---: | ---: | ---: | ---: |
| Default/Raid Target | 20261107 | 23.75 s | 6.65 s | 0 |
| Default/Dynamic Adds | 20261108 | 23.12 s | 7.61 s | 0 |

The ordered top five agreed in both cases, including a tie in the fixed
point estimates. These are two fixture runs with different final iteration
counts. They measure shortlist agreement on those inputs, including an
actual cull. Near-tie recall on other gear pools remains unmeasured.

A separate offline timing check used 49 distinct public Hunter gear sets,
45-second fights with three seconds of variation, and a five-set bulk
shortlist. Two exports of the same simulation inputs took 6.39 seconds
end to end. The ranker reused the first result and returned identical
rows for the second export. Changing only the second seed from 20261107
to 20261108 took 11.69 seconds and ran both profiles. These are single
CLI timings through the local MCP and pinned engine.

An offline follow-up held that gear pool, duration and duration variation
fixed and used seed 20261109 for both `Default/Raid Target` and
`Default/Movement`. Both profiles came from the local MCP catalogue. The
fixed pass ran all 72 sets at 1,000 iterations each. Bulk screened 71
candidates at 1,000 iterations for Raid Target and 1,030 for Movement, then
advanced five to 10,000 in each profile. The ordered top five agreed with
the fixed pass in both profiles and the same sets won in this fixture.

| Preset | Fixed wall time | Bulk wall time | Fixed top five missed |
| --- | ---: | ---: | ---: |
| Default/Raid Target | 24.49 s | 6.73 s | 0 |
| Default/Movement | 24.98 s | 6.82 s | 0 |

A separate cutoff check used 48 chest alternatives from the public Hunter
fixture, one target, three seconds of duration variation and 1,000 fixed
iterations per distinct gear set. It ran each profile with both `--top 5`
and `--top 10` and checked whether either shortlist contained all five of
the fixed run's highest-ranked distinct alternatives.

| Duration | Seed | Staged top five | Staged top ten | Fixed top five missed by either shortlist |
| --- | ---: | ---: | ---: | ---: |
| 45 s | 20261107 | 6.88 s | 29.87 s | 0 |
| 45 s | 20261108 | 6.86 s | 28.73 s | 0 |
| 90 s | 20261109 | 5.87 s | 25.52 s | 0 |

The pinned engine already uses uncertainty when deciding which candidates to
keep. With five requested results it screens against the leader, including
equipped gear. With ten it screens against the tenth candidate, giving more
sets a chance to advance. On this fixture that cost over four times as much
without recovering an additional fixed-run top-five set. Keep the default
for this workload. These results measure shortlist membership against a
finite-iteration reference. A larger gear pool or another encounter can
change the trade-off. [Pinned cutoff rules](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/bulk/statistics.go)

An offline follow-up ran the same 48 chest alternatives with a 2,000-iteration
paired reference per gear set. Both MCP encounter exports used 90 seconds
with five seconds of variation and seed 20261110. Movement was set to 12
yards every seven seconds. Bulk screened all 48 alternatives and refined
five in both encounters. The two modes returned the same ordered top five.

| Preset | Fixed wall time | Bulk wall time | Bulk screening / finalist iterations per set |
| --- | ---: | ---: | ---: |
| Default/Raid Target | 12.26 s | 3.43 s | 1,000 / 10,000 |
| Default/Movement | 12.05 s | 3.46 s | 1,027 / 10,180 |

These are phase timings with different iteration budgets. The fresh-seed
confirmation ran separately with seed 20200001 and 2,000 paired iterations
per set. The leading alternative beat equipped gear, which was the overall
runner-up in both profiles. The estimated gains were 3.73 DPS (approximate
95% interval 2.64–4.83) on Raid Target and 3.42 DPS (2.50–4.34) on Movement.
The same gear won despite the lower DPS on Movement. This check covers one
Hunter rotation and one gear pool.

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
to the trusted local MCP executable and either `--snapshot-id` or
`--character NAME REALM`. The latter selects the newest valid saved capture for
the named Forever character and reports its ID and capture time. If the two
newest captures share a timestamp, use an explicit snapshot ID. Configure
the usual owner snapshot directory and pinned catalogue in the environment.
Repeat `--slot` or use `--all-slots` to compare combinations, including keeping the
current item in each slot. The command pages up to 256 matching captured locations
per slot and scans at most 1,024 combinations, including the baseline. It skips
combinations that reuse a physical source or leave a moved item's slot unfilled.
It checks up to 64 remaining changes by default, rejects larger searches and
sorts resolved results by mean DPS against the captured baseline. Use
`--max-drafts` to choose a different limit up to 128. The baseline is
additional. The existing comparison iteration and
time limits still apply.
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

Use `--set-id` to restrict alternatives to pieces of one pinned catalogue
set. This can be combined with `--item-id`, in which case a candidate must
match both filters. The complete equipped baseline is still included.
Each encounter and weighted result row counts occupied simulator slots belonging to that catalogue
set, including retained equipment. The response also reports catalogue piece
count and distinct pieces observed across storage. These are set membership
counts, with bonus effects evaluated by the simulator.

Use `--storage` with `equipment`, `bags`, `character_bank` or `account_bank`
to restrict alternatives before draft expansion. The equipped set remains
the baseline, and storage coverage still describes the full capture.
Use `--minimum-required-level` and `--maximum-required-level` to narrow
alternatives by their recorded equip level. Unknown levels are excluded
when a bound is supplied. The captured character's requirements still apply.
Use `--observed-enchant-effect-id` and `--observed-random-suffix-id` to
select owned variants by exact captured modifiers before paging. A zero
suffix matches only a link with an explicit zero.

Use `--all-slots --single-swaps` to compare one replacement at a time across
all simulator slots. This avoids the multi-slot combination search. A
two-handed replacement may still clear the off-hand. The draft and
simulation budgets still apply.
Use `--max-swaps 2` to compare singles and pairs across the requested slots.
This evaluates two-item interactions while skipping larger combinations.
The limit accepts 1–17 selected replacements. Higher limits still stop at
the existing 1,024-combination scan cap and the configured draft limit.
Use `--plan-only` with the template, snapshot and slot options to inspect
resolved draft locations, projected slot clears, exclusions and storage
coverage before committing CPU time. Omit engine and encounter arguments.
Both modes include `gear_search` counts for retained candidate locations,
combinations before draft validation, submitted alternatives and resolved
drafts. The distinct simulator gear-set count collapses identical inputs
and includes the equipped baseline.

The ranker checks the template's class and race against the saved stats when
it recognises the captured tokens. A known mismatch stops the run. Missing
or unmapped observations are reported as `unverified` in `template_identity`.
The ranker reviews up to eight drafts per MCP call when the local server
advertises `additional_drafts`. Each batch uses one saved snapshot and
keeps input order. Older servers receive individual reviews.

An offline Windows check used 71 chest and hands candidates plus the
baseline. One run per binary returned identical collection results in
14.15 seconds with the older debug server and 5.55 seconds with batching.
Timing covers snapshot and draft collection before simulation.

A separate before/after run on the same fixture compared the already
batched debug server with per-batch catalogue verification and indexed
item lookup. The collection took 11.05 seconds before and 0.90 seconds
after, with identical ordered results.

The check uses the pinned simulator enum mapping. The simulator request also
requires the template's specialisation to agree with its declared class.
[WoWSims Forever enum definitions](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/common.proto)

The MCP also excludes a proposed swap from simulation when the pinned item
record explicitly requires a higher level or excludes the captured class.
Slot-filtered owned-gear reviews expose the conflict before the ranker expands
combinations. Draft reviews and skipped ranker rows report the conflicting slots. These
checks use available snapshot stats and catalogue records. Full in-game
eligibility remains unverified.

The same check screens known armour, melee, two-handed and ranged types
against the captured class using the pinned simulator's class capabilities.
Missing types leave the check unknown. This does not infer trained skill,
level-specific armour unlocks. Separately, the ranker screens proposed off-hand
weapon swaps against the selected template specialisation's dual-wield rules.
It preserves shields, held off-hand items and the equipped baseline.
[Pinned WoWSims class capabilities](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/ui/sim/player/classes/capabilities_auto_gen.ts)

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
storage copies before applying the combination and draft limits. It keeps one
copy for a single slot, or two physical copies when both ring, trinket or
one-handed weapon slots are selected. Each skipped copy records its retained
replacement locations. Equipped items keep their separate locations.

The same option now collapses complete resolved simulator equipment drafts,
including alternatives identical to the baseline. Exact item order, IDs and
resolved modifiers must agree. Each skipped draft retains its physical
selections and `representative_review`; the first validated representation is
kept. With this option, `--max-drafts` limits distinct resolved alternatives,
excluding the baseline. Without it, the limit continues to count proposed
alternatives. `gear_search.max_drafts_basis` reports which basis was used,
and `equivalent_simulator_drafts` counts these draft-level collapses. The
1,024-combination scan limit still applies before full-draft resolution.
Reviews are processed in batches of at most eight without retaining every
unfiltered response. This reduces duplicate budget use; it does not raise the
distinct-candidate cap or establish complete gear legality.

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
Configured draft and 120-second comparison limits apply in bulk mode. Rebuild
the adapter and use its new SHA-256 before running bulk searches over 64 drafts.
The encounter's
requested iteration count and the fixed mode's aggregate iteration cap apply
to fixed mode, including the fallback when every set matches the baseline.

The standalone simulator and gear ranker report duration, duration variation,
target count, execute windows and the requested iteration count. Adaptive runs report actual
stage iterations separately. `encounter_settings_sha256` fingerprints the
validated encounter and simulator options, including target inputs and seed.
It excludes character, gear and template data.

Fixed `--paired` comparisons with at least 200 iterations report an
approximate 95% interval for each distinct alternative's DPS delta versus
the baseline. An interval containing zero leaves the direction unresolved.
These are individual comparisons without a multiple-comparison adjustment.
Tank comparisons also include damage taken and threat per second.
Fixed tank runs report a 95% Wilson interval for each simulated death
fraction, including zero observed deaths. It estimates uncertainty for
that gear set under the chosen simulator settings.
Healing comparisons include healing per second. An interval below zero
for damage taken indicates an estimated reduction.
Healing results also report mean realised fight duration and the share of
total simulated fight time spent waiting for mana when the engine provides
usable duration data. The engine exports aggregate mana downtime only.
The ranker continues to sort by the point estimate.
Fixed paired DPS runs with at least two distinct inputs also report
`observed_leading_dps_pair`, the observed gap and paired standard error
between the two highest estimated results. Identical inputs count once.
The pair is selected on the same samples, so the gap is descriptive.
Use `--paired --confirm-leading-seed` with 200–5,000 iterations and a seed
at least that many integers away from each original seed to rerun those two
sets only. The confirmation reports the original leader minus the
original runner-up on fresh samples, including when the order reverses.
It adds a fixed comparison per distinct encounter with two eligible sets.
In adaptive bulk mode the pair comes from the returned shortlist and
equipped baseline. The confirmation seed must precede the selection seed
by at least the requested fixed iterations because the adaptive run can
extend its seed range forward.

Repeat `--encounter-lookup` up to four times to rank the same saved gear drafts
separately for each encounter. Output uses an `encounters` array in input order,
with a one-based encounter index, duration, duration variation and target count.
Review IDs identify the same draft across profiles. A bulk shortlist can omit
different drafts in different profiles. Each comparison has its own iteration
and time limits. The command prints results after all comparisons succeed.

Multi-profile DPS output also includes `encounter_sensitivity`. For every
requested review ID, `gain_pct_by_encounter` lists percentage changes from the
equipped baseline in input order. `minimum_observed_gain_pct` and
`maximum_observed_gain_pct` describe the available values;
`changes_sign_in_observed_encounters` identifies gains in one profile and losses
in another. This field is null with fewer than two usable observations.
`leading_point_estimate_encounters` lists where the set ties for the highest
returned DPS estimate, including ties with equipped gear.

Shortlist omissions stay null and appear in `missing_encounters`; even a draft
missing from every shortlist gets a row. Zero-baseline or non-finite percentage
comparisons also stay null. `fully_compared` is true only when every profile has
a usable percentage for that draft. Weighted bulk runs calculate this report
after their fixed finalist comparisons. These are point estimates, not tests
of statistical significance or proof that a set is universally best. Repeated
profiles still occupy their original positions and are not independent evidence.

A public-fixture offline check exercised Raid Target and Movement through the
real local bulk adapter. Both chest alternatives remained below the equipped
baseline in both profiles; the report did not invent a winner change.
[Recorded report](../../docs/benchmarks/encounter-sensitivity-2026-11-08.json)

For a combined ranking, repeat `--encounter-weight` once per encounter in input
order. Use positive weights such as `--encounter-weight 70 --encounter-weight 30`.
The ranker normalises the weights and averages each draft's percentage DPS change
from its encounter baseline. This keeps a high-DPS encounter from dominating
solely because of its DPS scale. Each baseline must have positive DPS.
Without the bulk adapter every draft is evaluated in every encounter.
With the bulk adapter, each encounter screens the full draft pool. The
ranker then runs the union of the shortlists plus equipped gear at the
exported fixed iteration count in every encounter. The output includes
`screening_encounters`, `finalist_reviews` and the fixed finalist results
in `encounters`. `--top` caps the distinct alternatives returned by each
screen. The weighted ranking can miss a consistently strong set
that never makes any individual shortlist. Use fixed mode when the
weighted result needs to cover every draft. Fixed iteration budgets still
apply to each finalist comparison. If requested, fresh-seed confirmation
compares the leading pair in each fixed finalist ranking.
Within one command, matching encounter and simulator options reuse the first
result. The repeated row reports `reused_encounter_index` and keeps its own
weight and position. Different seeds or iteration counts run separately.

Add `--cache` to `python -m tools.run_forever_sim` or
`python -m tools.rank_forever_slot` to reuse simulation results between commands.
The key covers the full simulator request, engine checksum, metric modes and
cache format version. The engine checksum is verified even on a hit. A changed
seed, encounter, rotation, equipment or precision gets a separate entry.
Each returned fixed-run row reports `cache_status` as `hit` or `miss`.

This opt-in cache belongs to the local OS user, under
`%LOCALAPPDATA%\AzerothOperator\simulation-cache` on Windows. It stores numeric
summaries and hashed request keys, not character names, raw captures or full
requests. It holds at most 128 summaries of up to 512 KiB each; SQLite adds
storage overhead. Windows encrypts result bodies with current-user DPAPI and
binds them to the request key. Other OS users cannot decrypt or reuse them,
and native owner/access checks reject existing folders or databases granting
access to other users. Unavailable, insecure or corrupt cache data causes a
fresh simulation.
Omit `--cache` to bypass it. Bulk screening also uses the cache, with a separate
key covering the adapter checksum, full ordered request list and requested
shortlist size. Its summary reports `cache_status`. Changing candidate order or
duplicate placement creates a separate entry, preserving review IDs. Only
recognised numeric stage telemetry and stage labels are persisted. On a hit,
stage durations describe the original computation, not the current lookup.
Fixed shortlist audits, finalist comparisons and fresh-seed confirmation runs
continue to use their own exact-input entries.

An offline public Hunter check used three seeds, 1,000 iterations and a
180-second Raid Target fight. Three separate cold CLI calls took 0.406–0.430 s;
their matching warm calls took 0.173–0.182 s. Each returned identical numeric
results. This measures reuse of identical work, not faster first-time simulation
or native encounter accuracy. [Recorded measurements](../../docs/benchmarks/forever-cache-2026-11-08.json)

A separate three-seed bulk check used the public Hunter baseline, two chest
alternatives and a 45-second Raid Target fight. Cold processes took 1.262–1.669 s;
matching cached processes took 0.210–0.235 s. Shortlists, metrics and review IDs
were identical. These timings include Python startup and fixture loading but
not MCP collection, and measure this small pool only.
[Bulk reuse measurements](../../docs/benchmarks/forever-bulk-cache-2026-11-08.json)

On the 49-set Hunter fixture above, a paired comparison of Raid Target,
Movement and a repeated Raid Target export at weights 5:3:2 took 11.08 s
with staged weighting and 27.64 s with exhaustive weighting. Seven review
rows reached the final pass. Every finalist row and the weighted winner
matched the exhaustive result. Both modes reused the repeated export.

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

An offline shortlist audit compared 49 distinct chest gear sets including the
equipped baseline, using the same public Hunter template, 45-second fights
with three seconds of duration variation and seed 20261107. The fixed path
used 1,500 paired iterations per set. The adaptive path screened 48
alternatives and took five into the high-precision stage.

| Forever preset | Fixed wall time | Adaptive wall time | Fixed top five retained |
| --- | ---: | ---: | ---: |
| Raid Target | 16.65 s | 5.81 s | 5/5 |
| Dynamic Adds | 17.04 s | 7.13 s | 5/5 |
| Movement | 16.32 s | 6.57 s | 5/5 |

The leading alternative stayed the same across these profiles. Its adaptive
estimate fell from 789.51 DPS on Raid Target to 767.11 DPS on Movement.
The adaptive high stage used 16,785–18,401 iterations depending on the
profile. The two paths used different precision, and each wall time is a
single run. This audit measures shortlist agreement on this fixture.
In a separate full-command Movement run, setting Go's processor limit to
12 yielded 9.93 and 9.97 seconds versus 9.55 and 9.15 at 20, with the same
shortlist and DPS to eight decimal places. The ranker retains the engine's
default processor scheduling.

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

`material_capacity_by_item` shows the whole crafts each fixed reagent
supports on its own, for bags and all observed storage. Missing storage
stays unknown, and partial coverage can understate the available stock.
Each item also shows how many of the requested crafts its captured
stock falls short by, separately for bags and all observed storage.
The remaining reagent units are shown when that stock covers the requested
consumption. These are per-recipe estimates before any crafted output is
added. Shared batch demands are reported separately.
For ordered batches, each starting-material row also estimates units
remaining across observed storage after all requested consumption and
known minimum outputs. The estimate is conditional on completing the
batch and requires enough observed starting stock for that item in the
supplied order. Unknown outputs contribute zero.
`ordered_starting_material_coverage` shows whether all starting needs
fit in captured bags or all observed storage. A known shortfall takes
priority over missing data. A zero starting need needs no inventory
observation. Partial storage can leave an actual supply unobserved.
`ordered_material_checkpoint` counts consecutive recipes whose materials
are covered by observed storage and identifies the first recipe where
stock is unknown or falls short. Earlier recipes supply only known
minimum output. This checks materials in the supplied order.
The first uncovered recipe lists blocking reagents by item ID. It reports
starting units needed through that recipe after earlier consumption and
known outputs, plus the shortfall when stock was observed.
It also shows the bag shortfall and suggested withdrawals from observed
banks through that checkpoint. A withdrawal can cover part of a shortage.
When a merchant reagent plan is requested, its
`first_uncovered_recipe_budget` prices those checkpoint shortages using
the same saved offers and inventory wallet as the full batch budget.

Each reagent also includes its observed stack count and up to 20 captured
locations, with storage, container, slot and stack quantity. Totals include
all observed stacks. These are locations in the saved export.

Pass `inventory_snapshot_id` to review a saved recipe against a separate
inventory capture from the same character and exact client/build. It must be
at least as recent as the recipe capture. Reagents come from the recipe
capture, while counts, locations and storage coverage come from the inventory
capture. The response includes both capture times. Missing storage stays unknown.

Use `target_output_quantity` in place of `crafts` when the recipe has a
captured base output range. The review rounds up using the minimum output
per craft, and the usual 1,000-craft limit applies.

Supply up to seven `additional_recipes`, each with a saved `snapshot_id` and
either `crafts` or `target_output_quantity`, to plan multiple distinct recipes against one inventory capture.
The primary capture provides the inventory if no separate capture is supplied.
`combined_materials` sums fixed demand and counts the shared stock once.
It is null if any recipe has unsupported requirements. The plan uses captured
stock and does not credit outputs from other planned crafts.
Single recipes and batches report `shortfall_vs_bags` separately from the
shortfall across observed storage. Each combined material includes
`quantity_by_storage` from the shared capture. Uncaptured storage has a null
quantity, and an uncaptured bag section gives a null bag shortfall.

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
