# Historical bag cooldown inspection — 9 November 2026

## Intended result

Find relevant historical carried-item cooldown observations without returning
the entire captured section. Optional inspect_snapshot.bag_cooldown_filter
requires section bag_item_cooldowns and Forever; no argument keeps raw output.
See ADR 048 for fields, limits, conjunction and missing-data semantics.

The result remains in the original snapshot envelope with capture time, age and
live=false. Its filtered data has bounded rows, full captured/matched/unknown/
nonmatching counts, paging metadata and false current_usability_verified and
game_execution flags. No action, elapsed countdown, stock credit or current
player claim. Addon package/capture and storage schema are unchanged.

## Author checks

- `cargo fmt`: passed.
- `cargo test --locked --offline --lib bag_cooldown_filter_tests`: six tests,
  .02s after 9.51s compilation, all passed. Tests cover conjunction and false
  over unknown, unavailable-status selection, numeric ordering, paging-only and
  usize::MAX offset, malformed filters, missing versus empty and 4096 rows.
- `cargo build --locked --offline`: ordinary MCP rebuild passed, 14.29s.
- With `PYTHONPATH=.;tests`,
  `python -m unittest test_bag_cooldown_filter test_bag_item_cooldowns test_item_cooldown_lookup`:
  three actual local stdio MCP tests, .956s, OK.
- After adding the valid 64-ID boundary and reported-status/unknown short-circuit
  cases: `python -m unittest test_bag_cooldown_filter`: one test, .455s, OK.

Actual MCP assertions cover 11 explicit predicate combinations, including
unavailable rows with unknown timers, definite unrelated IDs suppressing unknown
counts, exact zero and maximum-u32 timer boundaries, numeric bag 2 before 10,
numeric slot 2 before 12, seven offsets including maximum usize, unchanged
pre-pagination counters, 4096-row capture bounded to 50 results, 19 invalid
filter objects, wrong family/section contexts, absent versus known empty,
unchanged raw inspection and original snapshot bytes.

Only synthetic captures in temporary owner-local stores. No real game,
character, external data, installation, deployment or addon package change.

## Independent review

One clean round with gpt-6-astra high Callers and Failure/guard lenses. Each
ran the six prebuilt Rust tests (.02s/.03s), all passed. Callers ran filter and
lookup MCP tests: two tests, .753s, OK. Failure ran the filter MCP test: one
test, .462s, OK. Callers additionally verified tool-schema discovery, rejection
of eight incompatible review contexts, mismatched source item rejection before
filtering and a subsequent valid request on the same process. No concrete fault
found. Native/deployment lens omitted because those operations remain deferred;
agent token usage unavailable.

## Plan advisory

Jev 1.14.0 request
`a5d2324304f173770dec8cb9a3c7314df579fc361b693610fbafeab03323c011`:
proceed .55 / revise .35 / insufficient .10; 601 input, 47 output, .297s.
The uncertain advisory was retained unchanged; explicit predicate examples,
bounded output and actual caller tests establish this limited query behaviour.

Final Jev 1.14.0 advisory request
`df6bef88325dccf24e55bf51bf0033f6e4bbd314ba40b2086e2a9f56501216cb`:
land .85 / fix .02 / insufficient .13; 634 input, 47 output, .273s.
Author disposition: land this bounded historical filter on scoped tests and
two clean reviews. No current game state, native action or full goal completion
is established; live testing remains deferred.
