# Saved loot discovery — 2026-11-09

Baseline ecff0c6. The previous goal turn made source progress by fixing the
spellbook guard. This unit adds historical loot discovery through the existing
MCP route; broad system coverage, native qualification and owned BiS remain open.
No game, installation, deployment, engine or external data lookup performed.

## Author evidence

- `cargo test --locked --offline --lib loot_observation`: 3 targeted tests passed,
  50 unrelated filtered out, .00s after 10.19s compilation. Checks tagged shape
  rejection, kind isolation, row-presence zero, money null and 64×u32 aggregation.
- `cargo build --locked --offline`: ordinary MCP rebuilt after Rust changes,
  16.89s.
- `PYTHONPATH=.;tests python -m unittest test_loot_observation_discovery test_inventory_requirements_discovery test_loot_requirements`:
  3 tests passed, 1.633s. Actual stdio MCP tests use only synthetic owner captures.
- Expanded new test with raw-currency zero/64-row totals and inventory conjunction:
  `python -m unittest test_loot_observation_discovery`: 1 passed, .961s.

Actual caller assertions cover historical/latest missing or shorter rows,
date-window latest, explicit case-insensitive character selection, profession
composition, sorted numeric slot references, item/raw-currency collisions,
independent characters' quantities, empty/missing/unknown/zero/money states,
64-row u64 totals for items and currencies, u64-max threshold, pagination and
unchanged unfiltered output. All nine new evidence flags stay false; another
owner's capture/name/path never appears and both stores' bytes remain unchanged.
Seventeen malformed filters and two unsupported families reject without leakage.

## Independent review

Two independent read-only gpt-6-astra high agents reviewed Callers and
Failure/privacy. One clean runtime round, no actionable functional faults.

Callers ran the new actual MCP test and three prebuilt Rust tests. A separate
synthetic actual-MCP probe confirmed optional schema exposure, invalid newer
capture exclusion (invalid_count=2), valid older retention under the existing
latest-valid rule, and inventory conjunction without loot stock credit. Source
trace confirmed family/range preflight precedes empty-store return; validation
precedes predicates; latest selection precedes feature filtering; sorting and
pagination follow matching.

Failure ran the same three Rust tests and actual MCP test, then independently
rejected 16 malformed filters and five hash-valid malformed stored captures.
Valid money retained null quantities and all nine flags false; private malformed
row content was not echoed. Reviewed unknown/max/zero, cross-capture pooling,
owner separation and family/tag/amount bypasses.

Reviewers used prebuilt binaries without independently compiling source. Author
compiled after all Rust changes; later edits only extended Python tests/docs.
Agent token usage unavailable. Hosted/native drift lens omitted because those
operations remain deferred. No new addon package is needed for this query unit;
previous uninstalled capture packages are not represented as deployed.

## Advisory receipts

Plan Jev 1.14.0 request
957c3c0a9642a82f8310afdb069b5f26c755b182c420c4da0d2b2aa9e6d6ea41:
proceed .77 / revise .16 / insufficient .07; 602 input, 47 output, .350s.
Advisory only; unknown/raw-versus-owned semantics retained explicitly.

Documentation read-back: Callers found no contradiction. Failure found ADR-054
incorrectly said list summaries retained capture age; list exposes captured_at
only, while age_seconds belongs to inspect. Removed the /age claim without
changing code. No tests repeated for the prose-only correction.

Doc correction triage: Jev 1.14.0 request
60f222b699580b71fae111488976f8c5a9f440a46d204cb18af94bd50a7bedb1:
fix .97 / dismiss .02 / insufficient .01; 387 input, 44 output, .293s.
Failure final read-back confirms corrected capture-time claim and evidence;
no remaining concrete doc fault. Callers lens was not repeated for this one-word
prose-only correction; no new caller code or field was introduced.
Reviewed ordinary MCP SHA-256:
6a3941247afde29290876e5faba0f196c56015daa74ebafd5ac9e35bd08719db.

Final Jev 1.14.0 advisory request
fc6c0eea1cf1a4fdf5c9b3301ba8b7aab03af8578a104101d97052721e495ad7:
land_bounded_unit .98 / revise .01 / insufficient .01; 637 input, 56 output,
.277s. Author accepts the bounded historical query on tests and independent
review; broad goal remains active. Final format/diff checks passed.
