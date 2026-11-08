# Filtered pet-view comparison — 8 November 2026

Source and offline verification only. No deployment, installation, game
interaction or native capture.

`compare_snapshots` now accepts `pet_view_changes: true` for Forever.
The response's `pet_view_changes` compares saved `pet_collection_view`
observations by species ID. Shared species with changed owned/unowned row
counts report before/after values and signed deltas. Species seen in only
one capture appear in numerically sorted `observed_only_before` or
`observed_only_after` arrays. A translated or renamed species does not
produce a count change.

The output keeps `coverage: "filtered_journal_species"`,
`equal_filters: "unverified"` and `collection_gains_or_losses: "unverified"`.
These are changes in displayed rows, not evidence of collection acquisition,
release or ownership loss. A filtered-out species is not a zero count.
If either section is unavailable, the result is null; an observed empty
view remains a real empty view. The key is absent unless explicitly selected.
No GUIDs, pet custom names or additional capture APIs are introduced.

The existing owner-scoped reader, same-character/realm/client/build and
capture-time ordering checks remain in force. The comparison also requires
Forever. Inputs use the existing strict section schema and canonical
species IDs; row totals are bounded by the 10,000-row capture limit.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_pet_view_comparison test_pet_collection_view test_profession_recipe_view
```

Build passed; three integration tests passed via actual local Rust MCP
stdio. Comparison cases cover positive/negative count deltas, numeric ID
ordering, translated names, missing versus empty views, opt-in output and
Forever/character/build guards. Existing capture search/validation and
profession recipe comparison tests passed alongside the new path.

After adding explicit realm and reversed-time cases, the new integration
test passed again. `cargo test --locked --offline --lib
analysis::tests::compare_identity_missing_and_changed` passed the existing
comparison identity test. Format and whitespace checks passed.

Independent caller and failure/security reviews ran on `gpt-6-astra`, high
effort, with no concrete finding. Caller probes verified advertised selector
schema, invalid option types, corrupted stored rows, maximum species ID and
signed count deltas. Failure probes rejected 27 malformed stored snapshots
and nine character/client/time/owner-path violations, retained null for
three unavailable-section cases without leaking reasons, and passed 40
seeded independent-oracle comparisons (seed 2917). Output omitted names.

Jev advisory scope assessment supported offline feature completion while
leaving the overall goal and native qualification open: `jev-1.14.0`, request
`e392945fb14a3bfb785395e42248e69a5cf749c3e58a82c827f36f631684c7f6`,
586 input / 51 output tokens, 0.325 seconds. Tests and actual caller probes,
not this advisory result, provide the verification evidence.
