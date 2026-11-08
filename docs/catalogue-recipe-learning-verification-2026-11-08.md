# Catalogue recipe learning — 8 November 2026

`lookup_forever_catalogue.only_observed_learned_recipe` can now discover crafted
item/enchant references from an owner-local saved `profession_recipe_view`, as
well as the saved selected schematic. Both join to a pinned catalogue crafting
source by its exact recipe spell ID. The existing recipe/profession filters must
still match that same crafting source. No recipe name matching is used.

For a matching source, a known true learning flag from either observation can
support the filter. Known true/false disagreement makes the combined result
unknown. A known selected output item ID that contradicts the catalogue output
also makes it unknown, even when list learning is true. If both observations
are false, the result is false. Missing/unlisted/null learning is unknown and
does not override a positive observation. Tracking alone is not learning.

Item acquisition crafted sources include `recipe_learning`:

- selected/list row presence and their independent nullable learned flags;
- `learning_conflict` for two known disagreeing flags;
- `selected_output_matches_catalogue`, nullable when either output is missing;
- combined nullable `learned` following the rules above.

The existing source `learned`, material capacity, yield, crafting conditions and
cooldown remain tied to the compatible selected schematic. They are not filled
from a list row. A filtered list is partial, and catalogue associations are
reference data rather than native proof of the recipe's output or craftability.
No profession enum equivalence is inferred between the native list profession ID
and the simulator profession enum. Recipe ID is the join key.

Enchant lookups use the same learning rule and their application item ID when
positive. Their existing `recipe_observation.snapshot_id` points to the capture.
An enchant without an application item has no output ID comparison. The filter
does not supply enchant target eligibility or reagent details.

## Offline verification

`cargo build --locked --offline` passed. Real stdio MCP tests use synthetic
owner-local snapshots and the existing pinned public catalogue; no download:

`PYTHONPATH=.;tests python -m unittest test_catalogue_recipe_learning test_profession_recipe_view test_recipe_skill_up`

Three tests passed in 6.554 seconds. The new test exercises missing, unknown,
explicit-null, false and true list learning crossed with absent/false/true
selected observations; selected-output mismatch; two learned list recipes in
one item search; a learned enchant source; exact source filters; and omission
of a synthetic private list name from output. Existing selected-only source
fields retain their old meaning, and list-only matches do not invent materials.

`cargo test --locked --offline --lib crafted_sources_` passed both existing
crafted-source regression tests. `cargo fmt --check` and `git diff --check`
passed. These are scoped offline checks, not a full client qualification.

Independent review lenses (`gpt-6-astra`, high effort): Callers and
Failure/security found no functional defect. Their real MCP probes included
exact enchant application/profession matches, unrelated selected recipes,
wrong recipe IDs, unknown/null selected output, conflicting learning and output
mismatches. Private synthetic recipe names were absent. Callers identified the
old selected-only documentation, which was updated to the new contract.
Its final documentation/test check was clean, with the updated real MCP test
passing in 2.934 seconds. Failure/security ran the new test and eight additional
synthetic MCP cases without finding an actionable defect.

The initial test helper accidentally used `key` both as a snapshot parameter and
a catalogue query keyword. Renaming the test helper parameter fixed the harness;
no product change was needed.

Jev plan advisory: `jev-1.14.0`, request
`1b763f64fc6069801545c7e788935f7cf8c8864bc8f2b814fa0e3b825f53dc9f`,
494 input / 47 output tokens, 0.313 seconds; proceed probability 0.91.
Advisory probabilities do not verify the join or qualify live game behaviour.

Post-verification advisory: `jev-1.14.0`, request
`104c17bb12d76058966e09c45de794d95f03a4200823ca8dc8398223a9689d29`,
537 input / 50 output tokens, 0.299 seconds; land-scoped probability 0.87.
The scoped tests and independent source/caller review establish the evidence.

No live capture, game action, installation or deployment was performed.
