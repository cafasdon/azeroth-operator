# ADR 043: shared captured spell-resource search predicates

Date: 9 November 2026. Status: source/offline; no deployment.

Resource observations are inspectable but cannot drive saved-capture discovery
or row selection. Add one strict `SpellResourceFilter` for inclusive current-
charge/cast-count bounds and enabled/active cooldown flags. Validate nonempty
predicates and ordered ranges. Conjunction is three-valued: any known false
dominates unknown; otherwise missing requested fields remain unknown. Only true
rows match; no missing number or Boolean is fabricated.

Embed optional `resources` in the existing saved spellbook filter. Permit omitted
spell ID only with resource predicates, preserving old ID-only behaviour. Existing
ID matching still selects a base-or-reported book row; its resources belong to
the explicitly projected reported ID. No alias-based casting inference is made.
Run all metadata/resource predicates on the same row before numeric projection.
Add resource fields/counters only when requested, preserving legacy result shape.

Expose the same filter for explicit spellbook inspection, after existing ID/name/
passive/off-spec selectors. Count unknown/nonmatching rows at that stage. Missing
books yield null counters, observed-empty books zero. Retain snapshot paging and
newest-per-character ordering; resource unknowns do not trigger stale fallback.

No capture, snapshot schema, addon, package, permission or game-action change.
Matching historical observations does not verify current castability, complete
spellbook coverage, active character or native API behaviour.
