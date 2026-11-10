# Saved recipe restock: offline evidence

Scope: ADR-087, source/offline route and synthetic saved records only.
Installation, deployment and live testing remain deferred.

Commands:

- `cargo build --offline`: passed, 22.31 seconds.
- `cargo test --lib recipe_restock --offline`: 4 passed, 91 filtered.
- `PYTHONPATH=.;tests python -m unittest test_recipe_restock_mcp`: 1 passed, 0.346 seconds.

Rust tests cover bag-only counts, conditional output, no invented craft when
stock is sufficient, same-item net yield, unknown inputs, family and craft bound.
Actual stdio SDK tests exercise route discovery/annotations/profile agreement,
selected and exact bulk recipes, yield ranges, bank exclusion, zero/empty bags,
same-item use, 1,000/1,001-craft boundary, malformed targets, missing bags/output,
unsupported inputs, conflicting bulk/selected outputs, wrong family, missing
recipe, owner/path isolation and unchanged snapshot bytes.

The first synthetic fixtures omitted the selected recipe's required name.
The Rust validator refused them; adding the fixture field made all checks pass.
No validation rule changed.

Plan advisory: Jev 1.14.0 chose implement (confidence 0.77); hash
c7df6008bd6a2c40469a2c51dadcbd9a25c6d1c29d435e1a13512a3ef0f84f42,
566 input / 45 output tokens, 0.301 seconds. Advisory is not native proof.

Independent round 1 (requested gpt-6-astra/high, failure and callers/data):
failure reproduced selected output item2 versus bulk item3 for recipe100.
Explicit ID rejected, but omitted ID succeeded. Root traced the existing
selected-path bypass. The new restock path now invokes exact bulk consistency
checks for known same-ID schematics, without replacing richer selected output.
Regression checks cover omitted/explicit conflict, bulk output metadata absent,
and budget-exhausted bulk with complete selected evidence.

Root also reproduced a tools.smoke_mcp exact discovery failure because its
expected list omitted the new route. Updated stdio/HTTP smoke expectations and
added an invalid-snapshot restock call to the stdio probe. This registers the
intentional read-only feature; no mutation capability or deployed config changed.
The older deploy/probe_gateway exact list already omits earlier routes; it is
a separate pre-existing deployment-probe gap, not a live acceptance result.

Jev triage: fix both (confidence 0.96 conflict / 1.0 smoke), jev-1.14.0,
hash 4c64d6eccd88318b620f4049a703c82b982b3f68e8e9b0b897f9034bcb635c80,
655 input / 89 output tokens, 0.294 seconds.

After fixes: offline build passed (8.41 seconds), Rust 4 tests passed,
MCP/profile 3 tests passed (0.415 seconds). Actual stdio smoke against an empty
synthetic store passed: 12 tools, 3 families, traversal denied, 9 invalid-snapshot
requests rejected, unknown mutation rejected. HTTP smoke was not run against a
live endpoint. No live activity occurred.

Independent round 1 also passed a seed870087 arithmetic oracle: 206 SDK cases,
69 accepted / 137 refused / zero mismatches. Two base-item variants aggregate
only as declared base IDs. Unlearned recipes retain false craftability checks.
Eight bad IDs/unknown fields rejected; stored null count rejected on read.
Caller/data probes covered arbitrary path/override/action parameters, owner
hash mismatch and continued responsiveness, with no directory disclosure.

Second review round: fresh gpt-6-astra/high failure and callers/data reviewers
found no defect. Failure ran 44 independent SDK calls: item/yield/reagent/status/
profession conflicts rejected both modes even when target already met; omitted
ID kept richer selected metadata where bulk optional output was absent;
unavailable/budget-exhausted bulk did not erase selected evidence. Positive,
same-item and zero-craft arithmetic and bank exclusion passed; fixture bytes
stayed unchanged. Two probe setup errors (unsupported status literal unknown,
missing selected name) were corrected without source changes.

Callers ran MCP/profile 3 tests (0.380 seconds), actual stdio smoke and an
independent SDK probe. HTTP source expectations matched the profile. Closed
schema and nine invalid/extra requests rejected, optional/null recipe ID retained
selected metadata, and the server remained responsive. No builds, network,
private stores, game actions or deployment occurred in review. Live-drift lens
was omitted because no live state changed.

Final advisory: Jev 1.14.0 chose ready for source/offline only (confidence 0.97),
hash 7a829226a1e405df660595c0954d048619245ce152db1a92c4d27579c7247873,
630 input / 44 output tokens, 0.275 seconds. Root accepts the bounded result
from scoped direct tests and a clean second review, not advisory probability.
Native acceptance, current identity, variant equivalence and full ownership
remain unproven.
