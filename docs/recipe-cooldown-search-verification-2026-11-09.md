# Bulk recipe cooldown discovery — 9 November 2026

Baseline `93655a1`. Scope: optional captured cooldown search predicates on the
existing owner-local Rust stdio MCP route. No addon/native change, installation,
deployment, external data lookup or live game test.

## Author checks

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 13.98s.
The real MCP was exercised with synthetic local snapshots:

```text
C:/Python314/python.exe -m unittest test_recipe_cooldown_search test_bulk_recipe_cooldown test_recipe_material_search test_recipe_view_search test_recipe_search_inventory -v
Ran 6 tests in 3.035s
OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. The new test alone passed
in 0.920s. Its independent Python oracle checks all 79 nonempty combinations of
state/minimum/maximum predicates, charges-before-timer classification, missing
numeric evidence, known-false precedence, inclusive zero/u32 boundaries, invalid
inputs, same-row material/view composition, numeric paging and a 4096-row capture.
It distinguishes missing/empty bulk data, compares review classification, rejects
selected/newer-inventory cooldown substitution, checks original timestamps and
source-byte immutability and absence of synthetic private markers in responses.

State equality is a comparison against a captured category, including `unknown`.
`cooldown_unknown_count` counts unevaluable numeric predicates without a known
false predicate, not all rows in the unknown category. Counters are evaluated
after primary ID/status narrowing and before later view/material predicates.
None of this verifies present readiness, full profession knowledge or craftability.

## Review and advisory receipts

Plan advisory: `jev-1.14.0`, request
`91a840936f210f795c3db41dd0114e0e55f6c128d568c8f40f20678013c8c48e`,
591 input / 47 output tokens, 0.281s. Proceed 0.71, revise 0.20,
insufficient evidence 0.09, confidence 0.56. Advisory only.

One review round, two independent `gpt-6-astra` high-effort lenses; actual agent
token usage was not exposed. Callers ran cooldown/search-inventory/view tests:
3 tests, 2.176s, OK. Independent stdio probes checked tool-schema discoverability,
exact output/reagent/status narrowing, zero-charge precedence, unsupported unknown
state, maximum-usize offset, counters and unknown classification. A malformed
initial custom fixture was correctly rejected and corrected for the probe.

Failure/security ran the new MCP test: 1 test, 1.026s, OK. Its independent inline
stdio harness passed 30 probes: 13 malformed hash-valid stored payloads, three
incompatible inventory captures, nine invalid filter/path requests and five
boundary/three-valued/counter/paging scenarios. Source bytes stayed unchanged;
private sentinels were absent. Neither reviewer opened SQLite, built, changed
project files, used HTTP or operated the game. Both read ADR/docs and found no
concrete actionable fault; no fix/follow-up round was required.

Failure-review triage advisory: `jev-1.14.0`, request
`b893f00c65f9a520c08821c75a7445ed9f292bd81db527c58cf48579014f83c1`,
638 input / 48 output tokens, 0.283s. None material 0.92, fix required 0.02,
insufficient evidence 0.06, confidence 0.88; Callers was pending at that point.

Final scoped advisory after both reviews: `jev-1.14.0`, request
`a9ee2183776725d0aa791c0402ff0fd487606ca66ac55f9f3f3154c4eaf9bcfd`,
654 input / 52 output tokens, 0.262s. Land scoped 0.99, fix required 0.00,
insufficient evidence 0.01, confidence 0.98. These are uncalibrated advisory
judgements, not test evidence. Author disposition: land this bounded source-only
unit; native qualification and wider coverage work remain open.
