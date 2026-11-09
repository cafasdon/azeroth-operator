# Spell-resource discovery — 9 November 2026

Baseline `fb88b62`. Shared saved-data predicates for capture listing and spellbook
inspection. No addon/schema change, new package, installation, deployment, external
lookup or live/native test.

## Author checks

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 14.78s.

```text
C:/Python314/python.exe -m unittest test_spell_resource_search test_spell_resources test_spellbook_search test_spell_usability -v
Ran 4 tests in 13.633s
OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. New real Rust stdio tests
check 575 nonempty valid combinations across six predicates with an independent
Python three-valued oracle. Both inspection and listing match expected rows and
stage counters. Directed probes cover same-row ID/metadata composition, zero,
u32-max, unknown versus false, missing/empty/legacy captures, resource-only queries,
newest-capture unknown exclusion, a 1,024-row book and 20-row numeric projection.
Empty/null/unknown/reversed/invalid predicates, wrong section/family and absent
ID/resource selection reject. Names stay out of capture match projections and
source JSON bytes remain unchanged. Existing search/resource/usability checks pass.

## Advisory and review

Plan advisory: `jev-1.14.0`, request
`500e797476244d6f931296e93c240e438264be87761c8c554ef8c10c0a76333d`,
647 input / 47 output tokens, 0.302s. Proceed 0.50, revise 0.43,
insufficient evidence 0.07, confidence 0.25. The uncertain advisory is retained;
concrete tests and independent review determine the bounded disposition.

One independent review round, `gpt-6-astra` high-effort Callers and
Failure/security lenses, both clean. Actual agent usage was not exposed.
Callers ran the new resource-search test plus existing spellbook/usability/
learned-level tests: four passed in 14.286s. An independent actual-MCP harness
checked seven resource pages, base/reported row semantics, cross-row rejection,
name-scoped unknown counters, null legacy equivalence, schema discoverability,
private-name exclusion and immutable source bytes. Its first schema assertion
assumed inline fields instead of `$defs`; the corrected probe passed.

Failure/security ran the new test (13.386s), then 33 independent actual-MCP calls
covering same-row conjunction, reported-ID sibling isolation, scoped counts,
private projection, malformed/null/ID inputs, page limits and maximum unsigned
offset. Both readers' resource projections retained strict shapes; rejected
requests did not change the store. No concrete product finding or fix round.

Reviewers used the frozen parent-built executable and synthetic saved data,
without builds, project edits, network, native/game operations, installation,
deployment or SQLite. Parent build evidence establishes source correspondence;
no native qualification is inferred.

Final source/offline advisory: `jev-1.14.0`, request
`ef59051c402297299ba81304ea2209758b8c37040ff59616e426c4767fe0c66e`,
874 input / 52 output tokens, 0.292s. Land scoped 0.88, fix required 0.06,
insufficient evidence 0.06, confidence 0.82. Uncalibrated advisory only, with the
earlier uncertainty retained. Author disposition: land this bounded discovery
feature; native/current eligibility and the broad coverage goal remain open.
