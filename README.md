# Azeroth Operator

Local character analysis and player-operated configuration tools for World of Warcraft.

## Client subdivisions

- `clients/retail/`: Retail adapter, addon and capability evidence.
- `clients/classic/`: Classic adapter, addon and capability evidence. Classic variants must be identified explicitly; Era and progression clients are not interchangeable.
- `clients/forever/`: Forever adapter, addon and capability evidence.
- `shared/`: client-independent snapshot contracts, policy enforcement and planning.
- `mcp/`: shared MCP interface, requiring an explicit client identity.
- `tests/`: shared tests and separate client qualification suites.

Status: read-only stdio MCP foundation implemented and integration-tested. No live game client qualified yet.

`resolve_character` resolves an explicitly named saved character within the
configured owner's store and client family. Without an explicit character it
returns `active_identity_unavailable`, with no selection. Saved recency never
establishes current play; automatic active-character detection remains pending.
[Selection evidence](docs/character-resolution-verification-2026-11-08.md).

Forever exports can include `pet_action_bar`: partial native command/spell
slots with reported active, autocast and range flags. Inspect the section or
read `character_summary.pet_action_bar_observations`. Missing slots stay
unknown. Pet/spell names and target identities are omitted; the MCP issues
no commands or autocast changes.
These are historical observations, not pet identity or cast eligibility.
[Offline verification](docs/pet-action-bar-verification-2026-11-08.md).

Forever visible spellbook rows can include stable API `usability` flags.
Search saved rows with `spellbook_spell.usable` and `insufficient_power`, or
read `character_summary.spellbook_usability_observations`. Unknown flags do
not match false. These are historical, partial spellbook observations; they
do not prove current cooldown, target, range or cast eligibility.
[Offline verification](docs/spell-usability-verification-2026-11-08.md).

Forever linked-item captures can include stable `sell_price_copper` from the
native item API. `character_summary.captured_vendor_price_values` reports
quantity-based reference values separately for each observed storage, keeping
missing prices and overflow explicit. These values are not sale eligibility,
transaction proceeds or wallet funds; no item is sold.
[Offline verification](docs/item-sell-price-verification-2026-11-08.md).

Known native slot-category conflicts now withhold resolved Forever simulator
equipment, including conflicting baselines. Owned-gear reviews and ranking
report these conflicts separately; unknown or conditional categories do not
create a new blocker. Raw drafts remain inspectable. This is a conservative
simulation qualification check, not proof of live equip legality.
[Offline verification](docs/native-slot-conflict-verification-2026-11-08.md).

Forever linked-item captures can include stable `equip_location` tokens.
Equipment reviews annotate the selected item's native slot category, keeping
unknown tokens and conditional two-handed off-hand placement explicit.
A category match does not prove equip eligibility; overall compatibility
and simulation checks retain their separate rules.
[Offline verification](docs/native-equip-location-verification-2026-11-08.md).

Forever exports can include `weapon_enchants`, recording public enchant lists
for main hand, off hand and ranged slots. `character_summary` exposes the
reported enchant types, IDs, timers and charges with explicit unobserved hands.
Timers describe capture time; zero or missing details do not establish expiry
or effectiveness. No enchant is applied or cancelled.
[Offline verification](docs/weapon-enchants-verification-2026-11-08.md).

Forever item captures can include optional `native_stats` read for the exact
item link. Equipment reviews compare numeric tokens observed on both items
and keep tokens observed on only one side separate, without assuming zero.
Missing maps remain unknown; raw stat differences do not establish performance,
complete item effects or simulation compatibility.
[Offline verification](docs/native-item-stats-verification-2026-11-08.md).

Forever exports can include `equipment_durability`, with stable readable
current/maximum pairs for equipped slots 1–19. Inspect the section directly
or use `character_summary.equipment_condition` for per-slot percentages,
damaged/broken slot lists and the lowest observed percentage. Unobserved
slots remain explicit; this is partial coverage and does not establish
repair eligibility or price. [Offline verification](docs/equipment-durability-verification-2026-11-08.md).

Forever's visible local profession list can include stable reported recipe
difficulty, skill-up flags and skill-up counts. `character_summary` adds
`profession_skill_up_candidates`: up to 50 learned rows with a known true
skill-up flag, sorted by recipe ID, with totals and unknown counts.
`list_snapshots.profession_recipe` accepts `can_skill_up` and
`minimum_skill_ups_reported` alongside the exact recipe ID. Unknown values
do not match false or zero. These are historical observations from a filtered
list, not guaranteed gains or proof of craftability.
[Offline verification](docs/recipe-skill-up-verification-2026-11-08.md).

Forever flight-map snapshots can include optional `entries.<slot>.map_position`
with numeric `x` and `y` in `[0,1]`. Saved route legs expose these as
`origin_map_position` and `destination_map_position` when known. Coordinates
refer to that leg's recorded `map_id` and UI frame; they do not establish
world distance, travel time or movement instructions. Missing positions stay
absent, including on legacy captures. Fares and route selection are unchanged.
Only the open flight map at the recorded flight master is observed.
[Offline verification](docs/flight-map-positions-verification-2026-11-08.md).

Reachable destinations can also include `observed_route_node_ids`, a compact
object keyed `"1"` through `"N"` containing the origin, intermediate nodes and
destination from the panel's readable route preview. Saved route legs retain
this annotation when known. The preview describes one quoted flight; its
segments do not become separate itinerary legs or individual fare quotes.
Unknown, incomplete or changing paths stay absent.
[Offline verification](docs/flight-route-preview-verification-2026-11-08.md).

Hosted read-only MCP: `https://mcp.cafas.com.br/scopes/azeroth/mcp`, using a dedicated
bearer token. A separate restricted SSH credential supports the Rust CLI
`azeroth-operator upload <export-file>`. Configure the absolute
`AZEROTH_UPLOAD_SSH_CONFIG` path to its dedicated SSH config; normal administrator
SSH configuration is never used as a fallback. Uploaded bytes are validated and
confirmed by matching content hashes.

For repeatable incremental uploads, use `azeroth-operator sync <retail|classic|forever> <export-file> <receipt-dir>`.
Missing exports report `waiting_for_export`. Matching local acknowledgements skip
SSH connections; failed transfers remain retryable. Receipts bind the snapshot to
the dedicated SSH configuration contents. Keep that configuration self-contained
and use a different receipt directory if its destination changes indirectly.
A skipped upload does not recheck remote storage. Use `upload` to resend after
remote deletion or restoration. This command does not capture live game state.

`tools/sync_exports.py --config <absolute-json-path>` performs one bounded pass
over explicit account roots. Configuration contains absolute `binary`,
`ssh_config` and `state` paths, plus `clients` entries with `family` and `accounts`
(the client's `WTF/Account` directory). Only `SavedVariables/AzerothOperator.lua`
is read from each account. Linked paths are rejected. At most eight exports are
processed, with a process lock and per-transfer timeout. The latest result is
written atomically to `state/status.json`; it contains no account names or paths.

On Windows, `deploy/install_sync_task.ps1 -Config <path> -PythonWindowless <pythonw.exe>`
installs a current-user, limited-privilege task every five minutes and at logon.
The runner is copied to a hash-named local runtime directory. Pin the Rust binary
to a separate installed copy before scheduling; do not point production tasks at
an executable being rebuilt. The task runs only while the user is logged on,
does not wake the computer, rejects overlapping runs and has a ten-minute limit.
It never triggers exports or reloads: run `/aoexport` and reload/log out in WoW
to persist a new snapshot. Remove the task with
`Unregister-ScheduledTask -TaskName AzerothOperator-ExportSync -Confirm:$false`.

The companion is Rust; in-game adapters use Lua. Build with `cargo build --locked`. Run `target/debug/azeroth-operator.exe` for stdio MCP. Set `AZEROTH_DATA_DIR` to the existing absolute snapshot directory private to this installation’s owner. The MCP and local import CLI require this setting. Tools: `capabilities`, `inspect_snapshot`, `list_snapshots` (explicit family, optional offset/limit).

Import a JSON snapshot or the addon's narrow SavedVariables export with `target/debug/azeroth-operator.exe import <export-file>`. Import is a local CLI operation, not an arbitrary-path MCP tool. It reads data only; never executes Lua. See `addon/README.md` for the export workflow and outstanding live-client qualification.

Analysis tools: `character_summary` reports observed counts and unknowns; `compare_snapshots` requires the same character and exact client/build, rejects reversed capture order, and returns at most 100 changed JSON paths per section with truncation metadata. Missing sections are not treated as empty inventories. These are snapshot observations, not recommendations about optimal equipment.

Pass an optional `item_id` to `compare_snapshots` for that base item's before
and after quantities and signed change by storage. A storage change requires
both captures to include that storage. The net change requires all supported
storage sections in both captures. The comparison reports quantity changes
without inferring which individual copies moved.

`review_equipment_selection` creates a player-review JSON draft bound to one
`snapshot_id`. Supply 1–19 `selections`, each with `target_slot`, `expected_item_id`
and a `source`: either `{"kind":"bag","bag":0,"slot":1}` or
`{"kind":"equipment","slot":16}`. IDs must match the exact observed locations;
duplicate targets and reused locations are rejected. The draft shows previous
items, selected items and equipment slots that would be vacated. It does not
apply changes, emit a game import or claim compatible/optimal gear. Game legality,
requirements, capacity and current ownership remain unknown. Schema version 1
does not identify item GUIDs, account or region. This tool is included in the deployed seven-tool
server release; see the dated verification log for deployed qualification.

With a verified local Forever catalogue, an empty selection list reviews the
captured equipment as a simulator baseline.
Forever drafts can also reference observed character-bank and account-bank
locations. See the [equipment review guide](docs/equipment-review.md).

Synthetic performance baseline: `cargo run --release --locked --example benchmark` measures warm-cache listing and inspection over 200 generated snapshots. It uses no account data.

The 2026-11-03 paired Windows benchmark measured listing medians of 91.6ms before
and 69.2ms after borrowing item fields during validation and resolving the store
root once per listing. Each snapshot still receives path-containment, SHA256 and
schema checks on every call; no metadata freshness cache is used. Four alternating
pairs and all samples are recorded in `docs/benchmarks/listing-2026-11-03.json`.
This is a synthetic local result, not a claim about hosted Linux request latency.

The Python prototype is retained temporarily as a reference and independent protocol test harness. Install its test dependencies with `python -m pip install -r requirements.txt`. Set `AZEROTH_TEST_BINARY` to the Rust executable to test that server instead of the prototype.

For a local development addon ZIP: `python tools/package_addon.py --family retail --variant live --interface 120105 --version 0.1.0-dev`. Explicit client metadata is required. Generated `dist/` files are ignored; manifests include per-file and archive hashes. These packages are development artifacts, not certified compatibility claims.

For the pinned upstream packaging path, run `python3 -m tools.package_bigwigs`
with the same client/version arguments in Linux/WSL. It uses isolated staging,
disabled networking during packaging and exact output-file checks. See
[addon packaging](docs/addon-packaging.md) for prerequisites and the packaged Lua
fixture verifier. No release is published automatically.

## Boundaries

Read ordinary addon exports and official API data; analyse and prepare player-reviewed imports. No autonomous gameplay, computer use, memory access, injection, packet interception, protected-action bypasses or arbitrary Lua execution. In-game writes remain disabled pending operation-specific policy review and client verification. A callable API is not proof of Blizzard authorisation.

No character data, account identifiers, credentials or local exports belong in Git.

## Verification

Run `python -m tools.verify_source` for the local source gate after installing requirements.txt and the Rust toolchain. It checks formatting, builds the Rust binary, runs Rust tests and Clippy, then runs the Python suite against that exact target path. Each step has a ten-minute ceiling with process-tree cleanup on timeout; the first failure stops the gate. It does not package, install, upload or qualify a live game client.

Run `cargo build --locked`, `cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, and `python -m unittest discover -s tests -v` from the project root. The Python suite always exercises the Rust stdio server from target/debug (or the explicit AZEROTH_TEST_BINARY override), with a separate Python-reference test. Missing Rust binaries fail the test rather than silently using the reference implementation. These checks do not establish runtime game compatibility.

The current MCP source exposes an explicit offline-analysis allowlist. It refuses legacy tools that generated in-game Apply codes. Historical addon deployments and tests remain documented separately. See the [enforced operation policy](docs/operation-policy.md).


Inspect an explicit installation without launching the game:

```powershell
python -m tools.inspect_client --client 'T:\Games\World of Warcraft\_retail_' --family retail --variant live
```

The probe reads only `.flavor.info` and the parent `.build.info`, with byte limits and link/reparse rejection. Conflicting active versions are reported rather than selected. Family is checked against the product code; variant remains caller-supplied, interface remains unknown and installation metadata does not qualify live APIs. Classic accepts `wow_classic` and `wow_classic_era` products with an explicit variant; unknown products fail closed. Missing directories report `not_installed`; malformed or unreadable metadata exits non-zero. No account data is read.


Check installed addon integrity against a retained package manifest:

```powershell
python -m tools.verify_installed_addon --client 'T:\Games\World of Warcraft\_retail_' --manifest 'T:\Projects\Azeroth Operator\dist\bigwigs-0.1.5\AzerothOperator-0.1.5-dev-retail-live.manifest.json'
```

Exit zero means the four installed files matched the supplied manifest during this read. Missing, changed, extra or oversized files produce nonzero without repairs. The manifest is caller-trusted, not signed authentication; concurrent edits can invalidate the observation. Retail and Forever product checks are supported; Classic installation verification remains unavailable. No live API compatibility is implied.

Operation-by-operation scope, client subdivisions and policy evidence limits are recorded in [operation policy](docs/operation-policy.md). This is a project boundary, not Blizzard certification.


`character_summary.carried_item_review` reports observed carried quantities and repeated item IDs, with up to20 groups and20 locations per group. Totals cover the whole exported bag section even when details are truncated. Locations retain exported bag/slot keys. Different variants can share an ID: this is not item identity, stack compatibility, a move plan or an optimiser. Missing bag data remains null; an observed empty bag map reports zero. Hosted rollout was verified with a synthetic public HTTPS readback on 2026-11-03.
