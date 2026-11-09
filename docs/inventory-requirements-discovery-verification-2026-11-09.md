# Shopping/reagent list discovery — 9 November 2026

## Intended result and boundary

Find owner-local historical captures whose one explicitly selected storage
covers every requested item quantity. One-pass requested-ID counters; no item
payload retention, unrelated IDs or pooling across captures/characters/storage.
Original single-item filter remains unchanged and cannot be combined with the
requirements filter. See ADR 049 for request, metadata and unknown semantics.

Results are material observations, not current craftability or transfer plans.
Native ownership/access/shared scope, crafting, transfers and game execution
remain false/unverified. There is no active-character default or completeness
claim. No catalogue, external data call, addon change or deployment.

## Author evidence

- `cargo fmt`: passed.
- `cargo test --locked --offline --lib inventory_requirements_filter_tests`:
  nine tests, .00s after 8.71s compilation, all passed. Covers required fields,
  limits, duplicate IDs, positive quantities, family restrictions, all-item
  conjunction, missing/empty storage, equipment counts, checked u64 totals,
  request-order projection, malformed counts and no other-storage pooling.
- Existing prebuilt library binary with `inventory_item_filter_tests`: seven
  tests, .00s, all passed. Single-item filter behaviour is preserved.
- `cargo build --locked --offline`: ordinary MCP rebuilt, 18.52s.
- With `PYTHONPATH=.;tests`,
  `python -m unittest test_inventory_requirements_discovery test_inventory_item_discovery test_character_resolution`:
  three actual local stdio MCP tests, 1.199s, OK.

The new test uses two temporary owner stores and synthetic captures, with no
catalogue configured. It checks split stack totals, a partial list on different
characters, a partial list split across bags/bank, a complete list in one capture,
newer short/unknown captures suppressing older complete stock, explicit older
time bounds, captured profession/rank composition, equipment counting, 64-item
boundary, 12 malformed filters, conflicting single-item predicate, max u64
quantity, max usize paging, all client/storage scopes, duplicate shared-bank
observations kept separate, absence of the other owner's data/path, unchanged
ordinary listing shape and original snapshot bytes.

No real character, game process, crafting, transfer, installation or deployment.

## Plan advisory

Jev 1.14.0 request
`aa2782229c1fba7842020437edfcaef67b388a8bb2e687068c69cf9834047bfa`:
proceed .75 / revise .17 / insufficient .08; 646 input, 47 output, .284s.
Advisory only; direct tests establish the limited discovery behaviour.

## Independent review and disposition

One clean round with gpt-6-astra high Callers and Failure/privacy. Both ran nine
prebuilt requirements tests, passed. Callers ran requirements/single-item MCP
tests: two tests, .713s, OK. Failure ran new requirements (.402s) and existing
single-item (.404s) MCP tests, both OK. Callers also independently checked schema
discovery, real u64 totals above u32::MAX, preserved request order, exclusion of
invalid unrequested stock, no raw links and no catalogue requirement. No concrete
fault found. Native/deployment lens omitted because neither operation occurs;
agent token usage unavailable.

Final Jev 1.14.0 advisory request
`685d792535cd1be28d1ddf59577dc7fd5486f2159418d0ec2aa79c97786db2a6`:
land .89 / fix .02 / insufficient .09; 654 input, 47 output, .279s.
Author disposition: land the bounded material-discovery feature on direct
caller evidence and two clean reviews. The broader MCP goal, active identity,
complete ownership and native qualification remain open.
