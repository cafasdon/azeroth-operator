# Recipe restock source references: offline evidence

Scope: ADR-095. Synthetic owner-local captures and the actual local stdio MCP.
No provider query, refresh, external data download, native action or deployment.

Direct author commands:

- `cargo build --offline`: passed, 16.87 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_sources_mcp`:
  1 passed, 0.360 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_sources_mcp test_recipe_restock_quote_mcp test_recipe_restock_batch_mcp test_recipe_restock_mcp test_chain_captured_sources test_recipe_source_locations`:
  6 passed, 2.394 seconds. TEMP/TMP used C:/Temp/codex for legacy fixtures.
- `git diff --check`: passed with no output.

The new caller asserts shared demand once, requested producer output credit,
bank exclusion, gross single demand and zero-craft demand, exact provider/version
and capture metadata, unknown and truncated categories, known-empty categories,
scoped entity locations, missing source/item statuses, cyclic-order refusal,
composition with merchant quotation and individual-craft search, default/false
compatibility, strict boolean schema, malformed boolean and foreign-store
refusal, and unchanged synthetic capture files.

The first fixture failed the existing retained-entity coverage validator before
MCP execution: it omitted NPC IDs from the location cache. The fixture now
contains every required retained ID; status variants remove the cache rather
than retain contradictory entity coverage. No validator or guard changed.

Independent review round: callers and failure/privacy, each requested on
gpt-6-astra at high effort. Both found no defect. Callers ran the new SDK test:
1 passed, 0.394 seconds. Failure/privacy ran it independently: 1 passed, 0.225
seconds. Additional actual-executable SDK probes used temporary synthetic stores.

Callers confirmed selected recipe with omitted ID (shortage item 1:5), self-
recycling with stock schedule (gross item 10:8), multiple materials including
an absent bag item (1:5, 2:15), and zero-craft joint targets with both options
(empty source records). The schema, route, imports, helper and docs agreed.

Failure/privacy compared 36 shared-input cases against an independent integer
oracle with stocks [0,1,5,11], first targets [1,2,7], second targets [1,3,5].
All shortages matched. Further probes confirmed bank/mail exclusion, absence
of owner identity/bag link in the source projection, self-consuming gross
single demand, empty zero-craft source/location results, timestamp/provider
provenance and explicit empty versus unknown categories. A 33-NPC source with
32 retained location rows projected only the needed unretained NPC 100 as
`not_captured`, without unrelated coordinates.

Root judgement: ready for source integration within the saved-reference scope.
One clean round, two lenses. Paid Jev did not run under the user's spending stop.
Native/provider-current qualification and deployment remain deferred, not passed.
