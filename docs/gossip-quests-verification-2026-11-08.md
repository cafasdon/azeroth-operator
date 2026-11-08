# Visible gossip quest observations — 8 November 2026

Forever manual capture adds `gossip_quests` with coverage
`visible_gossip_quest_lists` and independently optional `available` and `active`
maps. Each map contains up to 128 positive u32 quest-ID keys. Rows can contain
public Boolean `repeatable`, `is_complete`, `is_trivial` and `is_ignored` flags.
At least one list must be observed; an observed empty map is distinct from a
missing list or unavailable section. Names, titles, dialogue, speaker identity,
GUIDs, coordinates and option text are not exported.

## Native source contract

The [Forever GossipInfo API definition](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/GossipInfoDocumentation.lua)
defines available/active list and count methods and `GossipQuestUIInfo`. The
[native gossip UI](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/GossipFrame.lua)
uses the global GossipFrame and the same quest IDs. Generated metadata spells
one flag `repeatable`, while the UI reads `isRepeatable`; capture normalises both
only when all present aliases are public Booleans and agree. Any restricted,
malformed or conflicting alias leaves that flag unknown; there is no fallback
that recovers a restricted value through a second field.

Capture requires an already visible GossipFrame before/after collection. Each
list is read twice, checking its native count before and after each read. Counts
must agree with dense returned arrays. Duplicate, invalid or restricted quest IDs,
missing APIs, unreadable values, exceptions, excess size or changed ID sets make
the affected list unavailable. Reordered arrays with the same ID set are accepted.
Optional flags survive only when two reads agree; failures do not invent false.

No frame is opened, refreshed, closed or selected. No gossip option, quest
acceptance, completion or reward API is invoked. The existing manual export
combat gate remains in force. The quest log is a separate observation source;
this feature does not cover QuestFrame greetings, all dialogue pages, all quests,
NPC identity/continuity, current eligibility or successful game transactions.

## MCP

Rust and Python require the exact Forever payload, canonical bounded ID keys,
nonnull Boolean flags and at least one list. `inspect_snapshot` returns the
section. `character_summary.gossip_quest_observations` returns null for a missing
section and null per missing list. Observed lists report full quest counts,
first 50 numerically ordered IDs, reported completion counts/first 50 IDs and
separate truncation flags, plus rows with unknown completion. False and absent
flags remain distinct. Acceptance and turn-in eligibility verification stay false.

## Offline verification

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test gossip_quests --test addon`: three Rust
  tests passed. The gossip Lua/Core fixture covers readable/reordered/empty data,
  secret/error/missing/duplicate/sparse/changed/count-mismatched lists, hidden or
  closed visibility, independently retained active data, unknown flags, unavailable
  API, combat refusal and forbidden selection/acceptance/turn-in/refresh stubs.
- The final alias and mid-read count-race fixture delta passed its targeted rerun.
- `PYTHONPATH=.;tests python -m unittest test_gossip_quests`: one real stdio MCP
  test passed. It checks missing versus partial/empty states, false/unknown flags,
  128-row and 50-ID bounds, raw inspection, Python/Rust malformed-payload and
  family rejection, and diagnostics without a synthetic private marker.
- Formatting and diff checks passed. No native capture, game interaction,
  installation or deployment was performed.

Independent Callers and Failure/security reviewers (`gpt-6-astra`, high effort)
found no concrete defect. Both ran the prebuilt Lua/Rust fixture and MCP test,
and reread/retested the final alias/count-race delta. Failure/security accepted
two valid boundary cases and rejected 32 additional malformed/private-field
Python payloads. Synthetic secret sentinels and reused binaries do not qualify
native restricted-value semantics; author compilation is separate source evidence.

Development package, not installed:
`local-data/gossip-quests-qualification/AzerothOperator-0.0.0-gossip-quests-test-forever-live.zip`.
SHA-256 `3641e76f0e8bbfba89d91f060098baa369bc375a96dda53e829c78d92c32ccf9`.

Jev plan advisory: resolved `jev-1.14.0`, request
`6426386e664a2f71df90efd35bddabaeb4d5b372e7c0affb9d9a33812aa86744`,
560 input / 47 output tokens, 0.327 seconds; proceed probability 0.88. Advisory
confidence does not prove native eligibility or successful game interaction.

Post-verification advisory: `jev-1.14.0`, request
`4b86accb64ee7143d9bae665d3a027895612cb24b7b4c2c6145d0e752d88b7df`,
603 input / 50 output tokens, 0.291 seconds; land-scoped probability 0.99.
Tests and independent source/caller checks supply the actual scoped evidence.
