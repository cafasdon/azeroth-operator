# Bulk recipe material comparison: offline evidence

Scope: ADR-094. Local synthetic captures and actual stdio MCP executable only.
Native testing, installation and deployment remain deferred.

Author commands:

- `cargo build --offline`: passed, 19.14 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_material_compare_mcp`:
  1 passed, 0.352 seconds; final schema assertions passed in 0.294 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_material_compare_mcp test_bulk_recipe_cooldown test_profession_recipe_view`:
  3 passed, 1.197 seconds. TEMP/TMP pointed at C:/Temp/codex for legacy fixtures.
- `git diff --check`: passed with no output.

The new actual SDK caller checks changed reagent/yield observations, status
transitions, known/unknown fields, cooldown/craftable-count exclusion, one-sided
rows, numeric order, full counts across pages, a page beyond 50, maximal uint32
offset, identical captures, missing sections, profession mismatch, old/null
response compatibility, malformed/redacted options, reversed chronology,
different character/build refusal, and unchanged synthetic capture files.
tools/list assertions bind the advertised closed object, uint32 offset and
1..50/default20 limit to the runtime contract.

Independent review round: callers and failure/privacy, each requested on
gpt-6-astra at high effort. No concrete defects found. Both reran the actual
SDK unittest: 1 passed in 0.272 seconds per lens.

Callers additionally ran 40 independent seeded oracle cases (seed 94) against
the actual executable. Counts, rows, changed fields, numeric ordering and pages
matched, including maximal uint32 ID, empty reagent maps, missing outputs and
all four statuses. Actual schema matched the documented nullable closed Page.
Valid Retail, changed realm, client variant and interface probes were refused;
both sections absent retained unknowns. An initial invalid Retail fixture kept
a Forever-only bank section; the corrected valid fixture tested the real guard.

Failure/privacy ran 12 additional actual-executable synthetic cases covering
Retail refusal, realm mismatch, foreign-store ID, traversal, absent sections,
numeric ID extremes, unsupported states, lost yield knowledge, hash-valid stored
invalid IDs/status/private fields, and malformed page types. Store/identity/
family failures returned `Snapshots unavailable, reversed or incompatible`.
Float offset and null limit returned the generic option deserialisation error.
Errors exposed neither the private marker nor temporary store paths. Binary
SHA-256 was A10A60194BB3805AF13DB1569694187668C609D212B85BC89F91C7573B4734A9.

Root judgement: ready for source integration within the explicit saved-observation
scope. One clean review round, two lenses. Paid Jev did not run under the user's
spending stop. No build/network/live operation occurred in either review.
