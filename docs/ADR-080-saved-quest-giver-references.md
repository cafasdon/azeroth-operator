# Saved quest-giver references

Date: 10 November 2026. Status: source/offline; no installation.

Extend the existing optional separately installed QuestieDB integration to
quest starters and finishers. Do not bundle its database or execute downloaded
provider code. Manual Forever export reads only IDs already in the same export's
saved quest log. Capture at most the numerically first 64 IDs, recording the
whole reported count (bounded at 4,096).

Require provider contract 1 and explicit Forever/source-or-baked metadata.
Retain contract version, not an invented dataset revision. Two equal projections
per quest and stable provider/getter/guard/context checks are required. An
existence miss is `not_in_provider`; unreadable or changing existence is
`unavailable`. A known record is `reported`, with unreadable or nil giver fields
omitted as unknown. Empty group tables are recorded separately.

Each giver ID group deduplicates at most 4,096 dense positions, retains the
numerically first 16 distinct positive uint32 IDs, and reports full distinct
count and truncation. Sparse positional outer giver groups are permitted;
inner ID lists must be dense. Starter groups cover NPCs, objects and items;
finisher groups cover NPCs and objects. No names, quest text, coordinates or
private provider errors are emitted. New `quest_giver_references` is strict and
Forever-only in both readers. Existing inspection and section-status discovery
expose it. Character summary counts statuses, preserving a missing section as
null and known zero as zero.

Both readers require the source quest section, matching reported total, and
exact numeric first-64 ID prefix. A reference section cannot assert coverage
over absent or different saved quests.

References are provider IDs, not verified native identity, current quest
availability, completion, route or interaction rights. No acceptance, quest
loading, UI selection, turn-in, reward or movement action is called. Data stays
owner-local and outside the public vector index. Update readers before exporter
installation. Native qualification and deployment remain deferred.

## Source assessment

Reviewed QuestieDB commit
[`c0ce0c97ee449295ff1a8356f815a713ff9e463c`](https://github.com/Questie/QuestieDB/commit/c0ce0c97ee449295ff1a8356f815a713ff9e463c),
committer date 10 November 2026 11:24:39 UTC. The
[public API](https://github.com/Questie/QuestieDB/blob/c0ce0c97ee449295ff1a8356f815a713ff9e463c/docs/api.md)
documents entity existence and keyed reads. The
[quest metadata](https://github.com/Questie/QuestieDB/blob/c0ce0c97ee449295ff1a8356f815a713ff9e463c/src/meta/questMeta.lua)
defines positional starter/finisher groups. The
[licensing notice](https://github.com/Questie/QuestieDB/blob/c0ce0c97ee449295ff1a8356f815a713ff9e463c/LICENSE.md)
records unresolved historic-contribution coverage. This reuses ADR045's
separately installed provider boundary; it adds no database redistribution or
claim of a new licence grant. Only API/schema/licensing references were fetched
for this assessment; no provider runtime or database was downloaded or executed.

Pinned documentation SHA-256:

| File | SHA-256 |
| --- | --- |
| docs/api.md | 8df9170a999c5b9368a19980a665967335bdfb28b2df0c9aa7ce8977b51ae00b |
| src/meta/questMeta.lua | d124338dbbc345503efb1f8cd50f7dd2d39a34760d445872f6b106af64441ee4 |
| LICENSE.md | d9937e385c401d234ef416b91f00290f8ea8e517f39ed4359aed99115770d8c9 |
