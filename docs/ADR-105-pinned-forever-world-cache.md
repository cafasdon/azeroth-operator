# Local Forever world reference cache

Use QuestieDB's own offline Forever Generation loader as the public world source.
The pinned revision, every staged file hash and the generated cache hash are in
`clients/forever/world.lock.json`. The existing WoWSims catalogue and owner-local
captures keep their namespaces and roles. This source adds quests, NPCs, objects
and items without translating provider IDs into native map/phase identities.

[QuestieDB's Forever notes](https://github.com/Questie/QuestieDB/blob/aca0da1c4a5ed0bc12997b9aaeef7b35af9ee26e/docs/forever.md)
describe independently maintained base data and correction layers. Use the
publisher's Generation path to apply Static Corrections and Derived Passes.
Do not treat raw Era rows or the uncorrected Forever seed as the same result.
Character-specific Dynamic Corrections, localized lookups and live client
availability remain outside this cache. The publisher documents limitations in
new Forever content. Counts describe this corpus, not world completeness.

QuestieDB's [licensing notice](https://github.com/Questie/QuestieDB/blob/aca0da1c4a5ed0bc12997b9aaeef7b35af9ee26e/LICENSE.md)
distinguishes historical contributions from specifically licensed content. Keep
the upstream notices and provenance in the local source pack. Do not commit or
bundle the publisher's assets, interpreter or generated dataset with this repo.
The lock and integration code contain no character data or third-party payload.

Run these commands from the project root on Windows:

```powershell
.venv/Scripts/python.exe -B -m tools.forever_world sync-source --source local-data/world-reference/pinned-source
.venv/Scripts/python.exe -B -m tools.forever_world build --source local-data/world-reference/pinned-source --cache local-data/world-reference/forever-world.jsonl
.venv/Scripts/python.exe -B -m tools.forever_world search --cache local-data/world-reference/forever-world.jsonl --kind quest --query Hogger
.venv/Scripts/python.exe -B -m tools.forever_world get --cache local-data/world-reference/forever-world.jsonl --kind quest --entity-id 176
```

Only `sync-source` reaches the network. It fetches one explicit revision from
the fixed public repository with sparse checkout, no credential helper and
bounded noninteractive Git commands. It verifies and publishes only pinned files,
not Git metadata. Reuse of a valid source pack performs no download. A mismatched
existing pack is refused. No automatic refresh, mutable branch lookup or fallback
data source is used by the runtime. For a future revision, review and replace the
pin and generated-output evidence together; keep old cache files under distinct
paths if needed.

`build` runs offline. It stages verified files, including the pinned Windows Lua
interpreter, with an isolated child environment and deadline. The Lua exporter
permits reads of staged pinned paths only and removes shell, native module and
write APIs from the generator's globals. These restrictions are defence in depth,
not an operating-system sandbox or trust in arbitrary Lua/binaries. The publisher
revision and pinned interpreter remain trusted dependencies. The writer caps
generated JSON lines and verifies the complete cache digest before publication.
Publication does not overwrite existing content. A matching existing cache is a
no-op. Cache verification is mandatory before serving.

`serve --cache <file>` starts a separate local stdio companion with two read-only
tools: `get_forever_world_record` and `search_forever_world_records`. Existing
server tool counts and installed profiles do not change. Exact lookup accepts a
kind and a positive uint32 ID. Search uses literal case-insensitive name terms,
optional kind, limit 1–20, offset uint32, and up to 128 UTF-8 bytes. Exact name
matches precede phrase prefixes, then other matches; ties use folded name and
kind/ID order. Fetch the exact record from a result's kind/ID. No embeddings,
network, source interpreter, snapshot access or writes are needed for queries.

Records preserve publisher field names. Nested tables use JSON objects with
decimal-string numeric keys, so nil holes are absent rather than renumbered and
zero/false remain values. This is an exact data projection, not a decoded
prerequisite evaluator. For example, the nested starter/finisher positions retain
the publisher's tuple semantics. Hashes cover each exact export line without its
line terminator. Responses cap at 256 KiB; unknown IDs return no record. Source
hash, cache hash, generation counters and record counts identify the evidence.
No current eligibility, spawn visibility, routing or native gameplay is claimed.

The exact MCP lookup accepts `include_quest_links=true` for quests. It
preserves source positions for grouped and alternative prerequisites, child
quests, group peers and exclusive quests. An unresolved ID retains a null
record. The view covers direct links and leaves quest eligibility unevaluated.
The same option adds `quest_contacts` for starters and finishers. It preserves
the publisher's tuple positions and resolves IDs in their NPC, object or item
namespace. Unresolved IDs retain a null record.

`quest_objective_targets` resolves the creature, object and item groups from
the original objective tuples. The raw fields retain quantities and other
objective data.

For items, `include_item_sources=true` resolves NPC drops, object drops,
container items, quest rewards and vendors. Each group has up to 32 entries
in numeric source-position order. Advance `item_source_offset` by 32 while
any group reports `has_more`. Each group reports its total count.

For NPCs and objects, `include_quest_links=true` resolves the cached
`questStarts` and `questEnds` fields to quest summaries with source positions.

For items, the same option resolves `startQuest` and `relatedQuests`.
The scalar starter appears at position `1`. The related quest array retains
its original positions. Item quest links and item acquisition pages can be
requested together.

For NPCs and objects, `include_item_sources=true` returns `sourced_items`.
Each group lists distinct matching items in numeric item ID order. The same
offset applies across groups, with up to 32 items per group.

For quests, the same option lists items whose cached `questRewards` field
references the quest. The page uses the same item ID order and offset.

For items, `sourced_items.itemDrops` lists cached items that name the selected
item as their container. The same offset also pages forward acquisition sources.

Quest links also resolve `breadcrumbs`, `nextQuestInChain`, `parentQuest`,
`breadcrumbForQuestId`, `availableUntilCompleted`, `availableStartingWith`
and `disabledByQuest`. Scalar references appear at position `1`.

`quest_objective_targets` also exposes named groups for `sourceItemId` and
`requiredSourceItems`. These retain the distinction between the item supplied
by the quest starter and additional items required by the quest.

The CLI get action supports the same views with --include-quest-links,
--include-item-sources and --item-source-offset.

Search accepts spawn_area_id to filter NPCs and objects by nonempty cached
spawn groups. A name query is optional with this filter. The CLI flag is
--spawn-area-id. IDs belong to the provider area namespace.

Search accepts quest_zone_or_sort for quests with a matching zoneOrSort value.
Use a positive provider area ID or a negative quest category ID. A name query
is optional. The CLI flag is --quest-zone-or-sort.

Search accepts max_required_level (1–255) to list quests whose recorded
requiredLevel is at or below the limit. Quests without this field are excluded.
Combine with a name or quest category as needed. The CLI flag is
--max-required-level. This filter covers the minimum level field only.

Search accepts min_item_level (1–65535) for items at or above the recorded
item level. Missing levels are excluded. The CLI flag is --min-item-level.

Search also accepts item_class and item_subclass (0–255). A subclass needs a
class. These exact numeric filters compose with names and minimum item level.
The CLI equivalents are --item-class and --item-subclass.
