# Bulk profession recipe materials — 8 November 2026

Source/offline feature; no native capture, game interaction, install or deployment.
Baseline `1a15f05cab30d993a677d8feb33006fe6b7d24b9`.
API provenance is pinned in `profession-bulk-data-audit-2026-11-08.md`.

## Contract

Manual Forever exports add `profession_recipe_materials`, with coverage
`reported_known_spell_schematics`, the native profession ID, and up to 4,096
entries keyed by reported known spell ID. This is independent of the existing
ID-only `profession_spells` contract and the selected-recipe planner.

Each entry has one of four statuses:

- `fixed_items`: a required `reagents` map, up to 64 item-ID/positive-quantity
  pairs; optional positive output item ID and minimum/maximum base output range.
  Repeated material IDs are added with checked u32 bounds. An empty map records
  no fixed required materials in the readable schematic; optional slots are
  excluded. This is not proof of craftability.
- `unavailable`: no usable matching pair of observations, including missing,
  restricted, malformed, unlearned, changed or throwing reads. No raw error text.
- `unsupported_required_slots`: required alternatives, variable quantities or
  currency requirements cannot be represented as fixed item quantities.
- `budget_exhausted`: an otherwise readable row would exceed 16,384 retained
  reagent pairs. Numeric ID ordering makes retention deterministic.

All non-fixed statuses contain only the status. Rust and Python reject extra
fields, null optional values, bad IDs/quantities, invalid output ranges, excessive
rows, excessive per-row materials, excessive aggregate materials and non-Forever
use. Historical source contracts remain unchanged.

The collector requires the existing local visible profession context and two
matching known-ID sets before and after the schematic reads. Each recipe uses
two matching public observations of learned metadata and its non-recraft item
or enchant schematic. One failed recipe does not discard successful neighbours.
Context/ID-set failure makes the whole section unavailable. The observations
are not an atomic transaction or evidence of unchanged native identity between
every individual call.

Only structured numeric references are retained: no recipe names, descriptions,
player identifiers or arbitrary API errors. No filter, selection, crafting,
protected API or gameplay operation is issued. The main manual combat gate and
4 MiB whole-export limit remain. Native availability and the responsiveness of
up to two per-ID metadata/schematic reads remain unqualified; offline tests do
not supply a native performance benchmark.

`inspect_snapshot` exposes the exact section. `character_summary` reports total
IDs and counts by status, with `complete_recipe_knowledge_verified: false` and
`craftability_verified: false`. A missing section gives null. This increment
does not supply bulk crafting plans, cooldowns, conditions or proof that every
known spell is a recipe.

## Author evidence

- Locked offline Rust collector test: one passed, 0.44 s. Covers summed duplicate
  materials, output range, independent failing neighbour, restricted values,
  changed quantities, malformed arrays, unlearned metadata, unsupported required
  choices/variables/currency, empty materials, empty known-ID list, changed
  profession/ID sets, hidden frame and deterministic aggregate budget boundary.
- Real stdio MCP integration and existing profession-spell integration: two
  passed, 2.425 s. Both strict readers reject malformed synthetic snapshots;
  summary/inspection preserve all status counts and null/empty distinctions.
  The synthetic owner store is byte-for-byte unchanged after reads.
- Existing addon exporter and profession-spell collector fixtures: three passed,
  including preservation of the previous export on failure and observed-empty
  versus unavailable storage.
- Author rebuilt the stdio executable from current source using the locked
  offline build before the MCP checks. Prebuilt test execution alone is not
  current-source or native qualification evidence.

Direct commands: `cargo test --locked --offline --test profession_recipe_materials -- --nocapture`;
`python -m unittest test_profession_recipe_materials test_profession_spells -q`
with `PYTHONPATH=.;tests`; and
`cargo test --locked --offline --test profession_spells --test addon -- --nocapture`.
The unchanged protocol smoke test also passed: one test, 8.143 s, using
`python -m unittest test_mcp.MCPTests.test_rust_stdio_server -q`.
Formatting and whitespace checks passed.

The development-only addon archive is
`local-data/bulk-materials-qualification/AzerothOperator-0.0.0-bulk-materials-test-forever-live.zip`,
SHA-256 `d1a3750667c07e0fb0029f449a632246b81d1ef3f47262858f34acfc8662ae8b`.
It was packaged, not installed or natively qualified.

Jev plan advisory: `jev-1.14.0`, request
`2cb35039841f8298de8131624f737417fc5a3ccf9feb6736504bede913cadc92`,
567 input / 47 output tokens, 0.302 s; proceed probability 0.92. Advisory only.

## Independent review and correction

Round one used independent `gpt-6-astra` high-effort Callers and Failure/security
reviewers. Callers traced capture, package projection, import, readers and MCP
outputs; its real MCP test, seven existing Python checks, prebuilt collector
test and in-memory family package probes passed with no finding.

Failure/security ran the prebuilt collector fixture, real MCP parity test and
190 additional synthetic Rust/Python verdict comparisons (13 valid), including
numeric extremes, malformed IDs, unknown private-marker fields and aggregate
budget boundaries. All reader verdicts matched and rejected marker content was
not echoed. It found a reasoned enum edge: absent recipe type and absent Enchant
enum constant compared equal as nil. The author added an `unknown_kind` fixture,
reproduced the incorrect `fixed_items` row, then required a numeric recipe type.
The same fixture now passes with the row unavailable: one test, 0.41 s.

Triage advisory: `jev-1.14.0`, request
`1eee03736f2608f7af8d66407ed6daaca45ee5e46b0b57f33a95f234f0895a2a`,
486 input / 45 output tokens, 0.270 s; fix probability 0.98.
An earlier pre-final-review advisory supported landing (request
`a7ebb2a3d44e0b12302c276df1d78eccf7511231ae6a211ef7affd8df267b9e0`,
556 input / 49 output tokens, 0.287 s, same resolved model). It was superseded
by the concrete finding and did not authorise ignoring it.

An additional maximal-width synthetic snapshot with 4,096 rows and 16,384
retained material pairs validated at 571,912 compact JSON bytes. This bounds
that example, not the combined size of every possible snapshot section.

Round two Failure/security review inspected the guard fix and ran the updated
prebuilt collector fixture: one passed, 0.45 s; no further finding. Callers was
omitted in round two because the fix changed no additional callers or contracts.
Post-correction advisory: `jev-1.14.0`, request
`7baf5bc2fd06bb8073e334245afd455b60cfd4eaecb447376292802ce867b8d7`,
506 input / 49 output tokens, 0.291 s; land-scoped probability 1.00. Advisory
confidence is not verification; the reproduced failure, regression and reviews
are the evidence. Broad MCP coverage and bulk planning remain incomplete.
