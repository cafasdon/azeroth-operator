# Reported recipe craftable counts — 9 November 2026

Base source: `06d85dd`. ADR 023 defines optional historical count capture and
projection independently of material and eligibility decisions.

## Public contract

Inspected the Forever UI mirror at immutable revision
`9dc84f0d3bbae1768ac3405b3716000bd93eda06`:

- [Generated TradeSkillUI API](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUIDocumentation.lua)
  defines `GetCraftableCount` with recipe spell ID, optional recipe level and a
  numeric available-count result.
- [Recipe-list implementation](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.lua)
  calls it with recipe ID alone. This is the capture's default-level basis;
  neither source establishes inventory scope or current native availability.

Public source research used the network. All capture and MCP verification below
used local synthetic data; no live game, installation or deployment was used.
Bulk cooldown/charge integration remains open; absence from inspected generated
documentation is not evidence that every native cooldown API is unavailable.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 9.00s
cargo test --locked --offline --test profession_recipe_materials
1 passed; 0 failed; test duration 0.33s
```

The MLua collector test exercises counts zero, seven and u32 maximum, plus
missing API, nil, negative, fractional, oversized, Boolean, secret, infinite,
NaN, error and changing results. Invalid counts preserve fixed material rows;
unavailable and material-budget rows receive no count calls. Private error and
value markers stay absent. Existing profession/context/material guards remain.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_reported_craftable_count test_profession_recipe_materials test_bulk_recipe_planning test_recipe_search -v
```

Four tests passed in 2.150 seconds. Strict Python and hash-valid Rust snapshot
readers reject malformed/nonfixed counts. Actual local Rust stdio MCP reviews
cover omission, zero, one, seven and u32 maximum, requested-craft saturation,
unchanged material review output and raw search projection. A later inventory
override retains the original recipe count and timestamp. Reported zero does
not block a mathematically supported chain, stock plan or quote; craftability
remains false. Capture files remain byte-identical.

Native API behaviour, count accuracy and responsiveness of up to 8,192 additional
reads remain unqualified. No policy-compliance guarantee is inferred from source
inspection or mocked execution. Live testing stays deferred.

## Independent review

One round with two independent `gpt-6-astra` agents at high effort: Callers and
Failure/security. Neither found an actionable defect; no correction round was
needed. Source remained frozen during their read-only probes.

Callers ran the two count/strict-reader Python modules: two tests passed in
1.045s. Direct compiled MLua test execution passed in 0.44s. Additional actual
Rust MCP probes confirmed target output seven with minimum yield two derives
four crafts, including primary/additional count deltas. Batch and dependency
outputs were invariant for absent/zero/maximum observations after removing only
new observation metadata and differing content-addressed snapshot IDs. A bags
material filter requiring ten crafts retained both supported recipes even when
the reported count was zero. Its first comparison included capture IDs; fixing
that probe normalisation passed, with no product change.

Failure/security independently ran the compiled MLua test (one pass), the new
MCP test (0.288s) and strict-reader parity test (0.753s). A further inline actual
MCP probe rejected 17 malformed/status cases in both readers without private
marker disclosure, including integral floats, negative-zero float, huge values,
objects, arrays, NaN/Infinity and counts on each nonfixed status. Target five
with minimum yield two derived three crafts independently of zero/maximum
reported count; nine invalid craft requests still rejected.

Both traced historical provenance and no-gating semantics and checked the
updated documentation. Neither used HTTP, a game client, installation,
deployment, builds, SQLite or Git mutation. External source research was not
independently fetched by them under their no-network brief.

Scoped Jev advisory: `jev-1.14.0`, request
`9fcdf7e27eb80f130fb1d24e1804ffe385e078e0c5df57ce2cd54c802eab902f`,
682 input / 52 output tokens, 0.311 seconds; land-scoped 0.99, fix-first 0.01,
insufficient-evidence 0.0, confidence 0.98. Advisory only, not proof or approval.
