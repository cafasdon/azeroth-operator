# Saved-character inventory discovery — 9 November 2026

## Result and boundary

list_snapshots now accepts inventory_item quantity bounds in one explicit saved
storage. Results retain per-character/capture identity and compact quantity
evidence; they never aggregate account-bank or other quantities across captures.
See ADR 049 for the request and evidence contract. No active/current inventory,
complete ownership, present access or transfer action is qualified.

## Author verification

- `cargo fmt`: passed.
- `cargo test --locked --offline --lib inventory_item_filter_tests`: seven
  tests, .00s after 7.57s compilation, all passed. Covers required/unknown fields,
  family and bound guards, missing versus empty storage, equipment counts,
  u64 stack totals, malformed counts and no other-storage aggregation.
- `cargo build --locked --offline`: ordinary MCP rebuilt, 15.62s.
- `PYTHONPATH=.;tests python -m unittest test_inventory_item_discovery test_character_resolution`:
  two actual local stdio MCP tests, .503s, OK.

The new test uses synthetic characters and two separate temporary owner stores,
without catalogue configuration. It checks positive and zero bounds, missing
storage, newer zero/unknown suppressing old positives, case-insensitive explicit
character selection, historical time bounds, all four storage scopes, Retail/
Classic support for equipment/bags and refusal of bank queries there, duplicate
account-bank amounts retained separately, no 100+100 aggregation to 101,
maximum u64 predicate, paging including maximum usize, ten malformed filters,
unchanged ordinary listing shape and unchanged original snapshot bytes.

An initial privacy assertion incorrectly treated an SDK diagnostic echoing an
unknown caller-provided field name as stored-data disclosure. The fixture was
corrected to assert absence of the separate owner's name and path value;
unknown request fields still reject, without changing SDK or runtime guards.

## Independent review

One clean round, two gpt-6-astra high lenses: Callers and Failure/privacy. Both
ran the seven prebuilt Rust tests, passed. Callers ran two MCP discovery/
resolution tests (.493s), OK; Failure ran the discovery test (.359s), OK.
Callers independently probed schema discovery, invalid newest capture exclusion,
newest valid feature mismatch suppressing older stock, explicit historical time
selection and no catalogue requirement. No concrete fault found. No native or
deployment lens because neither operation occurs; agent token usage unavailable.

## Advisory plan

Jev 1.14.0 request
`906446205c89a9bd78380fb238b0118b9574f6fbcd5b4347bde8a2756b027308`:
proceed .73 / revise .21 / insufficient .06; 648 input, 47 output, .298s.
Advisory only; direct source/caller checks establish this bounded feature.

Final Jev 1.14.0 advisory request
`b03ff28cf5cd54c031e84988739726555868bd72621052560f6c48db30e07c3f`:
land .91 / fix .04 / insufficient .05; 661 input, 47 output, .301s.
Author disposition: land this bounded historical discovery feature on actual
caller tests and two clean reviews. Active identity, complete ownership, native
access, transfers and the broader MCP coverage goal remain open.
