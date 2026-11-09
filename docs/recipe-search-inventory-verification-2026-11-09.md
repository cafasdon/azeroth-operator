# Recipe search with newer inventory — 9 November 2026

Scope: owner-local MCP recipe discovery with explicitly selected newer stock.
Baseline `5720bf9`. No live game testing, installation, deployment, dataset
download or external lookup occurred.

## Author verification

`cargo fmt` and `cargo build --locked --offline` passed; build 14.99s.

With `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`:

```text
python -m unittest test_recipe_search_inventory test_recipe_material_search test_recipe_view_search -v
Ran 4 tests in 2.045s
OK
```

These tests use the actual rebuilt local Rust stdio MCP and synthetic private
owner stores. The new case keeps three original recipe rows while the newer
capture contains no recipe rows and contradictory learning evidence. Old bags
have one reagent; newer bags have six. A two-reagent recipe becomes discoverable
for three crafts using the explicit inventory, while the original learned and
skill-up flags remain authoritative for that search. Individual recipe review
and discovery agree on material capacity.

The old capture's character-bank stock is absent from the newer capture and is
not carried forward. Missing and empty bags retain distinct unknown/insufficient
counts; zero-reagent recipes remain independent of stock. Numeric pagination,
equal-time inventory acceptance, omission/null legacy equality, timestamps and
snapshot IDs are asserted. Older, other-character/realm, family, variant, build,
interface and missing/malformed IDs reject. Inventory without material filtering
rejects. Generic responses omit synthetic private identity markers. Source JSON
bytes are unchanged. Craftability, complete knowledge, live and game flags stay
false. Existing discovery tests cover 4,096 rows, large material totals and the
three-valued recipe-view filter matrix.

The first synthetic Retail mismatch fixture incorrectly retained Forever-only
recipe sections and was rejected by the existing importer before MCP execution.
Removing those sections made it a valid Retail fixture for the intended identity
guard test; no production guard was changed.

## Advisory receipt

Independent `gpt-6-astra` high-effort Callers and Failure/security reviews found
no patch issue. Both ran the four focused tests against the actual binary
(1.957s/2.022s). Callers additionally checked the optional string/null schema,
same-ID provenance-only extension and inventory captures without recipe sections.
Failure checked traversal, uppercase/missing IDs, hash-valid malformed inventory,
argument shapes, invalid limits and UTC-offset ordering/equivalent instants.
Both confirmed no old-bank merge and preservation of source bytes.

Failure noted existing framework errors can reflect invalid caller-supplied enum
values/unknown field names. That is distinct from stored-owner-data disclosure,
is not introduced here and did not justify a runtime change. Operational failures
for validly typed requests remain generic. No reviewer changed files, built,
deployed, accessed external data or operated the game.

Plan: `jev-1.14.0`, request
`9fd3fd0bdaec3ff92fa16b09bebb3f9f13eb9bcd4dc52e8362d61aa9f7bea8aa`,
566 input/47 output tokens, 0.260s; proceed .90, revise .07,
insufficient evidence .03, confidence .85. Advisory only, uncalibrated.

Final scope/observation: `jev-1.14.0`, request
`f6b3264adf2c1eb4649f5a160760eac133eed36c7e80b5c9d19d9e3bfbd5a052`,
736 input/95 output tokens, 0.279s. Scoped landing .75, fix first .04,
insufficient evidence .21, confidence .62. Existing caller-input reflection:
no material patch issue 1.00, confidence 1.00. Uncertainty remains recorded;
these values are not proof or permission.

## Limits

Both captures remain historical. Matching identity does not establish present
learning, skill gains, cooldowns, accessibility or craftability. Filtered recipe
views and missing storage are partial evidence; complete profession/native
qualification and the broad MCP coverage goal remain unfinished.
