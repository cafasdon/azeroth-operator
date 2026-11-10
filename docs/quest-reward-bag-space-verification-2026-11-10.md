# Grouped quest reward bag space: offline evidence

Scope: ADR-099. Synthetic owner-local captures and the actual debug stdio MCP.
No private capture, external data lookup, installed change or game action.

Author commands:

- `cargo build --offline`: initial build passed in 20.16 seconds; final request
  error text build passed in 6.85 seconds.
- `cargo test --offline --lib quest_turn_in`: 12 passed, 87 filtered, 0.39 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_quest_turn_in_bag_space test_quest_turn_in_rewards test_quest_turn_in_review`:
  3 passed, 2.283 seconds. TEMP/TMP used C:/Temp/codex for old fixtures.
- `cargo test --offline --lib quest_turn_in::tests::reward_output_cap_rejects_large_valid_source_without_truncation`:
  final output-cap regression passed, 0.62 seconds, 98 filtered.
- `git diff --check`: passed.

The actual caller checks schema/default/closed-object behaviour, shared quest
deduplication, combined item counts, page-independent union, separate group
views, specialised-slot exclusion, missing/conflicting stack metadata, missing
bags/capacity, known empty rewards, unknown fixed lists, choice exclusion,
ownership/placement limits, malformed/redacted requests and foreign-store
refusal. Saved capture bytes stay unchanged. A 72-case independent integer
reference varies two reward quantities, carried stack headroom and free slots.

The first bag-capacity fixture omitted empty positions 2..4 and was rejected by
the existing Python validator before MCP. It now includes the complete required
five-position capacity record; no validator or guard was relaxed. The Rust
output-cap regression also covers the new bag-space opt-in beyond all group
pages, proving an oversized unique union is rejected without a partial result.

Independent review used the built-in reviewer agents, with no external inference
purchase or usage reset. Account identifiers and private usage/billing values are
excluded from this record.

One review round, callers and failure/math/privacy, each requested on gpt-6-astra
at high effort: both clean. Callers reran the actual MCP test (1 passed, 1.449
seconds) and existing reward/grouping tests (2 passed, 0.680 seconds). Ten
independent real-MCP guard/filter/group probes confirmed omission/null, source
scope, pre-page filtering, unique reward totals and matching per-group reports.

Failure/math/privacy reran the new actual MCP test (1 passed, 1.463 seconds) and
the compiled output-cap regression (1 passed, 0.40 seconds). Twenty-two additional
actual-MCP calls covered fourteen malformed option cases, filters beyond all
pages, all-unknown fixed lists, missing source sections and maximum valid reward
quantities. Sixty-four rows of quantity 4294967295 summed to 274877906880 and
13743895344 estimated slots, matching the independent integer reference. Invalid
source quantities were refused. No private marker or temporary path appeared.

Root judgement: ready for source integration within the documented saved fixed-
reward model. No runtime fix was needed after independent review. Native reward
delivery, item-variant stacking and placement remain unproven. Live testing and
deployment remain deferred; the broad system-coverage goal remains incomplete.
