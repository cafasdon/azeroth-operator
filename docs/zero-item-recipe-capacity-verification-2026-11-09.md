# Zero-item recipe capacity — 9 November 2026

Baseline `f50af96`. Source/offline correction to recipe preparation; no addon,
schema, installation, deployment, external lookup or live game test.

## Regression and author verification

The actual Rust stdio MCP reproduced an explicit empty fixed recipe with zero
charges and missing inventory. Combined capacity was null instead of a known
zero limit/three-craft deficit. The regression failed:

```text
None != {'supported_crafts': 0, 'shortfall_vs_requested': 3}
('free', 'missing', 0, 'bulk', 'bags')
```

An earlier exploratory run stopped on the absent new basis field. The selected
unsupported fixture was corrected to the existing `unsupported_required_slots`
plus empty-reagents contract before testing its behaviour.

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 7.69s.

```text
C:/Python314/python.exe -m unittest test_recipe_zero_item_capacity test_bulk_recipe_cooldown test_recipe_pending_mail test_chain_charge_accounting test_reagent_chain_stock -v
Ran 5 tests in 2.482s
OK
```

Python environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. The new actual-MCP
matrix covers 96 cases: free/required/unsupported materials, missing/empty/bags/
bank inventory, absent/zero/one/u32-max charges and selected/bulk recipe sources.
Both storage scopes are checked against an independent arithmetic oracle.
Finite material fields, charge capacity/shortfalls, basis, false verification
flags, absent synthetic character markers and unchanged source bytes are asserted.
After adding a target-output request with newer stock and a conflicting newer
charge count, the expanded test passed in 1.162s: three requested crafts retained
one original charge, two-craft deficit and separate capture timestamps.

## Advisory and review

Plan advisory: `jev-1.14.0`, request
`0ebeae29065068e81744324f0c58013a4f6d0b1a7cf180f064bee5485d0c5012`,
531 input / 47 output tokens, 0.299s. Proceed 0.85, revise 0.13,
insufficient evidence 0.02, confidence 0.77. Advisory, not test evidence.

One review round: two independent `gpt-6-astra` high-effort lenses, both clean.
Actual agent token usage and individual test elapsed times were not exposed in
their final reports. Callers ran the 96-case matrix and:

```text
C:/Python314/python.exe -m unittest -v test_bulk_recipe_cooldown test_recipe_reagent_chain test_bulk_recipe_planning test_recipe_cooldown
5 tests passed
```

Its independent inline actual-MCP probe checked single, chain/stock and batch
modes: original zero charges remained authoritative over newer inventory with
four charges, yielding capacity zero/deficit three. Missing additional-recipe
charges stayed unknown and captures were byte-identical.

Failure/security ran the matrix, then independent actual-MCP probes rejecting
11 malformed captures (missing/null/array reagents, invalid quantities and
malformed charge pairs). Four inventory provenance calls accepted compatible
newer stock and rejected wrong character, wrong build and earlier captures.
Character markers were absent and source bytes unchanged. Reviewers used the
frozen ordinary debug executable, without building, editing, network/native/game
operations or SQLite. Their matrix runs preceded the final extra provenance
assertion; the author ran it and Callers inspected it. No fix round was needed.

Caller qualification: selected-only acquisition catalogue projection still
exposes only legacy finite material capacity (`src/forever_catalogue.rs:1943`),
so its null reasons remain indistinguishable there. Bulk catalogue nested reviews
retain the full result. No runtime consumer was found inferring readiness from
the legacy null. This unchanged projection is not a regression; broad catalogue
enhancement remains open alongside native qualification and full profession work.

Final scoped advisory: `jev-1.14.0`, request
`93a5e1f6ded30765ea083b2401b70f9cfbde605527df83aa8f0af06d94c572a4`,
796 input / 52 output tokens, 0.281s. Land scoped 0.96, fix required 0.03,
insufficient evidence 0.01, confidence 0.94. Uncalibrated advisory only. Author
disposition: land this bounded source/offline correction; overall goal stays open.
