# Saved turn-in groups: offline verification

10 November 2026. Source/offline only; ADR081 defines the complete feature.
No game, provider query, database sync, installation or deployment was used.

`inspect_snapshot` accepts optional `quest_turn_in_review` on the
`quest_giver_references` section. It groups retained provider finisher IDs from
the same validated owner snapshot, replacing `data` only when requested.
Quest IDs/readiness select quests; kind-qualified giver filters select edges.
NPC/object namespaces, numeric order, readiness unknowns, omitted references,
truncation, ungrouped quests and pagination remain explicit. Missing sections
retain unknown groups; known empty remains zero. Verification flags for current
readiness, turn-in eligibility, native identity, complete coverage and optimal
routing stay false.

## Author evidence

- `cargo test --lib quest_turn_in --offline`: seven tests passed. They exercise
  shared givers, namespace collisions, false/unknown readiness, AND selection,
  numeric ordering, paging, missing/empty sections, uncaptured source quests,
  truncation and invalid options. The worker corrected positive fixtures to
  supply the real quest schema's title/level and validate each base snapshot;
  deliberately malformed cases mutate those validated fixtures afterward.
- `cargo build --offline` passed in 35.56 seconds. It includes the current
  production code; only test-fixture metadata changed afterward.
- `$env:PYTHONPATH='.;tests'; python -m unittest test_quest_turn_in_review`:
  one real stdio MCP test passed in 0.574 seconds. It checks 19 deterministically
  sorted groups, independent NPC/object ID 10, ready/not-ready/unknown IDs,
  source-status counts, duplicate-free page reconstruction, uint32 beyond-end
  offset, multiple AND filters, false excluding unknown, unknown requested
  quests, missing source/reference sections, known empty, selected uncaptured
  IDs 65/66, 66 ungrouped quests with 64 retained diagnostics, strict invalid
  options, wrong section/family, coverage conflict, foreign-owner rejection,
  excluded private quest titles and unchanged owner/foreign snapshot bytes.
  Raw inspection without the option remains unchanged.
- `git diff --check` passed.

A deterministic module worker requested gpt-6.1-sol medium with fresh context;
root implemented MCP wiring, integration test and documentation. Resolved model
and usage were not exposed. The worker performed no review, commit, deployment,
network access or game operation.

## Independent review

One clean round: failure and callers/security-data lenses requested
gpt-6-astra high with fresh context; resolved models/usage were not exposed.
Failure review passed the actual stdio test in 0.472 seconds, 81 independent
Python-oracle filter/readiness/namespace combinations, 18 paging boundaries,
21 malformed page/type/ID probes, 10 correctly hashed malformed source files,
four incompatible selectors and zero-match truncation diagnostics. It checked
numeric ordering through object/10 and uint32-max offset. An initial probe
incorrectly expected optional null readiness to reject; the existing schema
permits null. Corrected probes confirmed unknown treatment and exclusion from
both true and false selections. No source correction was needed.

Callers/security review passed the new actual MCP test in 0.346 seconds and
the existing raw quest-giver MCP test in 0.165 seconds. Independent tools/list,
null/omitted compatibility and 17 invalid/cross-selector/store probes passed.
Wrapper fields are unchanged; hash-tampered captures fail before summary with
generic error and no private sentinel. Both owner reads validate the same
snapshot content hash. Reviewers traced the actual input/router/store/review
path and changed docs. No actionable finding remained. The unrelated existing
branches of the large inspection handler were not exhaustively reviewed.
No reviewer source writes, builds, network, game operations or deployment.

Jev 1.14.0 advisory selected ready, confidence 0.73, request
`6c8c6dd70709907005c76408bfe70bd92228d31cf0bfad904eaa59e15a0b8f6e`,
684 input/44 output tokens, 0.389 seconds. Author accepted the bounded feature
on exact owner-local MCP results, independent oracle coverage and clean lenses.
This does not establish native readiness, provider/world identity, complete
finisher coverage or route quality. Live testing/deployment remain deferred.
