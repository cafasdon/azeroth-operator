# ADR-055: Explicit queried talent allocation and simulator preparation

Date: 2026-11-09. Status: offline source integration; native testing deferred.

## Missing outcome

Active talent entries alone cannot establish zero purchased ranks for every
unlisted simulator definition. Build preparation must not fill missing evidence
with optimistic zeros. Extend the existing observations to query all entries in
the active config's returned tree/node lists, then require an exact pinned class
definition match before generating any simulator digits.

## Optional captured allocation

Forever `talents.definition_allocation` is an all-or-nothing optional group:

```json
{"coverage":"queried_config_tree_entries","queried_node_count":1,"queried_entry_count":1,
 "definitions":{"123":{"node_id":11,"entry_id":111,"tree_id":10,"purchased_rank":0,"max_rank":5}}}
```

This illustrates the group shape, not a complete importable snapshot. Queried
node count must equal observed_node_count; queried entry count equals the number
of unique definition rows. Bounds: 4096 nodes, 4096 total entries, 64 entries per
node, 16 existing tree IDs. All IDs are positive u32; purchased rank is u32 and
at most the positive native maximum. Definition-map keys are canonical IDs.

Entry IDs and definition IDs are globally unique. A node belongs to one tree;
every row's tree must be in observed_tree_ids. Every active entry must have a
matching node/entry/tree row. Positive purchases require that active entry;
existing optional active definition tuples must exactly agree with the group.
The Rust and Python readers both enforce these linkage and budget invariants.

The collector reads public entryIDs, entryIDsWithCommittedRanks, ranksPurchased
and GetEntryInfo. Positive purchases require one unambiguous committed entry
matching the active entry; unselected entries are zero only from this explicit
node allocation evidence. Zero purchased points remain zero even when active
ranks are boosted. Unknown values, subtrees, duplicated IDs, ambiguous choices,
missing active membership or budget violations omit the whole optional group.
Stable base observations remain usable.

Two normalized allocation observations agree before emission; the second refreshes
all queried nodes, including inactive nodes. Required context and GetEntryInfo
identity are checked around the allocation reads and before the final whole base
graph recheck. All optional calls occur before that final base graph. This is
bounded sampled observation, not atomic native state or fresh live identity.
No names, native import text, UI loading, talent purchases or commits are added.

Primary field reference is the pinned
[SharedTraits API documentation](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/SharedTraitsDocumentation.lua).
That reference does not establish actual client availability or model truth.

## Explicit MCP preparation

```json
{"snapshot_id":"<explicit saved content hash>","prepare_simulator_talents":true}
```

Use `captured_talent_loadout`. Preparation is mutually exclusive with an explicit
simulator_talent_string comparison. The new Boolean accepts only actual Booleans;
invalid flag diagnostics are fixed text and do not echo supplied values. Omit it
or set false for previous behaviour. Retail/Classic cannot use this mode.

The nested simulator_talent_preparation result can emit
`status: prepared_for_snapshot_review` and a simulator_talent_string only when:

1. Class is known from the selected owner capture.
2. The optional queried allocation exists and passes strict validation.
3. Its definition ID set exactly equals the pinned class set, without missing or
   extra definitions, and every native maximum equals the pinned maximum.
4. staged_changes_observed is explicitly false.

The pinned nine-class map orders ranks into its three simulator trees, preserving
explicit zeros and using purchased rather than boosted active ranks. No class or
specialisation override, native import or point spending. Missing or mismatched
evidence returns unavailable with fixed reason and bounded numeric diagnostics,
without a string. Extra-ID diagnostics stop at 64 and report truncation/count.

The native import status remains separately unavailable for Forever. Legality,
native tree completeness, full allocation verification, model accuracy, native
import generation, game_execution and live remain false. Exact queried-set
mapping is not proof that native APIs enumerate every relevant mechanic or that
the capture reflects the character currently in play.

The draft may be placed in an explicit simulator template for review. Existing
class/race/profession/pet/level/gear/encounter/engine guards still apply. The
preparation query itself does not invoke an engine, alter a template or auto-select
a rotation. The current Python simulator's strict active-node projection remains
unchanged; it is not silently extended to infer full allocation coverage.

## Verification boundary

Offline Lua and actual MCP tests cover all nine class maps, explicit zeros,
ambiguities, staging, limits, reader parity and privacy. An opt-in actual pinned
Hunter engine check compares seeded metrics using the prepared string against
the public reference template. These are synthetic captures and one engine/model
example. Native acceptance/responsiveness, complete build mechanics and active
identity remain open. Matching reader/importer and uninstalled addon 0.50.0-dev
are needed to use the new capture field.
See [dated evidence](talent-allocation-verification-2026-11-09.md).
