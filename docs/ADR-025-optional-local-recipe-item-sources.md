# ADR 025: optional local provider references for recipe reagents

Date: 9 November 2026. Status: source/offline implementation.

The pinned simulator item catalogue omits common reagents. Extend manual
Forever capture with optional `recipe_item_sources`, using only the public API
of a separately installed QuestieDB provider. No provider code/data, network
sync, hard addon dependency or installation is introduced. The source integration
contains independent consumer code; provider rights and redistribution are not
relicensed by this project. Upstream licensing caveats are recorded in the
dated evidence; no bundled database is proposed on an assumed blanket licence.

Core passes the already successful/serialisable bulk material section to this
collector immediately after capture. Gather its fixed-row reagent IDs once,
deduplicate/sort numerically and query the first 64. `reported_item_count`
counts all distinct captured fixed reagent IDs, up to the existing 16,384 budget;
the difference from retained entry count is the omitted item count. This is not
full profession/world coverage, and omitted items cannot be treated as absent.

Require public contract 1 compatibility and explicit Forever source/baked status.
Check status before and after, and require the same provider table. Missing or
invalid context makes the section unavailable without breaking other exports.
No newest-snapshot identity or other-character default is inferred.

Each item is read twice under protection. Stable existence false yields
`not_in_provider`; failed/changing projection yields `unavailable`; stable true
yields `reported` with optional source categories. Each category reads an ID
array, requires at most 4,096 dense positions of public positive u32 values,
deduplicates and retains the eight smallest numeric IDs. Preserve the distinct
reported count and truncation. Category errors/secrets/invalid values omit that
category without erasing other valid evidence. Nil stays omitted because the
provider normalises empty arrays to nil; an explicit valid empty array is kept.

Retain NPC drops, object drops, container drops, quest rewards and vendors as
source-ID maps. No item/NPC/player names, coordinates, logs, external URLs or
free-form provider error strings are exported in this section. Existing owner
store identity and bounded strict schema apply in both readers. Inspection uses
the existing `inspect_snapshot` route, not a new game-command tool.

Provider contract/mode and export time are historical provenance, not a dataset
revision or evidence that a source is live, obtainable, owned or policy-qualified.
These references do not change stock, craftability, quotes or gear ranking.
Source-only tests use API doubles and synthetic captures. Native acceptance,
performance and full world/acquisition planning remain open, with live work
deferred. Joining this section into chain/route analysis is a further task.
