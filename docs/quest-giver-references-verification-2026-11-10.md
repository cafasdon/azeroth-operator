# Saved quest-giver references: offline verification

10 November 2026. Source/offline only. Feature contract and pinned API/schema/
licensing references are in ADR080. No game, installed provider, deployment or
automatic database sync was used. This is not native quest availability proof.

## End-to-end source result

Manual Forever export can consult an independently installed compatible
QuestieDB provider for starter/finisher IDs of the numerically first 64 saved
quest IDs. Both strict readers require the same export's source quest section,
equal whole count and exact numeric prefix. Bounds, omitted unknowns and
provider misses survive storage and existing MCP inspection, discovery and
summary. Nothing enters the public vector index or receives ownership credit.
No quest load, selection, acceptance, completion, reward or movement is invoked.

## Author verification

- `cargo test --lib quest_references --offline`: six module tests passed after
  the source guard fix. Valid zero/maximum, strict shapes, IDs, statuses,
  truncation, missing/mismatched sources and 66-quest prefix are covered.
- `cargo test --test quest_giver_references --offline`: two Lua 5.1/Core export/
  strict Rust decoder tests passed, latest 0.14 seconds. They cover 66 saved
  quests with 64 captured, duplicate/reversed giver IDs with first-16 retention,
  NPC/object/item starters, NPC/object finishers, missing provider records,
  changing existence, nil/error/malformed fields, restricted values, metadata/
  getter changes, empty quests and combat refusal. Forbidden action count stays
  zero and private provider errors are absent from serialized snapshots.
- `cargo build --offline`: fresh main binary passed after the source fix,
  14.29 seconds.
- `$env:PYTHONPATH='.;tests'; python -m unittest test_quest_giver_validation test_quest_giver_mcp`:
  15 tests passed, 0.456 seconds. Storage, strict malformed-input rejection,
  Forever-only scope, source-prefix matching, actual stdio inspection, summary,
  missing/empty distinction, section discovery, latest-before-filter behavior,
  foreign owner rejection and immutable capture bytes are asserted.
- Author repeated missing source, mismatched IDs and wrong-prefix cases through
  Python validation and fresh Rust `import-stdin`: both reject all three, with
  no files written to the disposable store.
- `git diff --check` passed.

Implementation was split between two deterministic workers (requested
gpt-6.1-sol medium, fresh context): Rust reader/summary and Python reader/tests;
root owned exporter, Core wiring, Lua/MCP integration and documentation. Actual
resolved models and worker token usage were not exposed. No worker deployed,
committed, fetched external data or performed review on its own work.

## Review and corrections

Independent failure and callers/security-data lenses requested gpt-6-astra
high with fresh context (resolved models/usage not exposed). Initial reviews
confirmed the fixture tests but found readers could accept reference coverage
over absent, different or wrong-prefix source quests. Author reproduced missing
and mismatched acceptance, then added cross-section source validation to both
readers and rejection tests. During recheck, the newly supplied positive source
fixture used empty quest rows, unlike the real Rust schema. Author reproduced
actual MCP rejection and corrected those fixtures to include title and level;
no production quest schema was relaxed.

Jev 1.14.0 advisory triage selected fix for the source guard (confidence 0.91,
request `3a60301900eadd44abc310d608b453a57a47d20620ea3fe9b69c6791c88e167d`,
511 input/46 output tokens, 0.347 seconds) and fixture (confidence 0.97,
request `8e86844fed72f658ccf19fed990280650a9df19691398ea91832818dcecd6736`,
455 input/46 output tokens, 0.301 seconds).

Clean follow-up failure probes show both readers agree on seven cases: absent,
contradictory IDs, wrong prefix and 4,097 quests reject; valid prefix, 4,096
quests and corrected fixture accept. Actual MCP test passed in 0.293 seconds.
Callers/security follow-up passed all 15 Python/MCP tests in 0.298 seconds and
independently confirmed valid 66-quest inspection plus correctly hashed invalid
files failing inspection/summary. Discovery excluded all four malformed files
with invalid_count 4. Owner-boundary and content-tamper probes passed. It also
verified all three cached upstream reference hashes and API group positions.
Neither follow-up found another actionable defect. Reviewers did not write
source, build, download, operate the game or deploy.

Jev 1.14.0 advisory shipping judgement selected ready, confidence 0.92,
request `d34b7a1152502b0d3902f21915dc0208a075db30337afa69a23b8e933966136e`,
744 input/44 output tokens, 0.323 seconds. Author accepted the bounded feature
on the real export/readback evidence, matching source guards and clean
independent follow-ups. Installed-provider semantics, native quest identity,
current availability and complete world coverage remain unproven. Installation,
deployment and live testing remain deferred.
