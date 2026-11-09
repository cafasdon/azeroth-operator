# ADR 037: filter original bulk recipe cooldown observations

Date: 9 November 2026. Status: source/offline; no deployment.

Bulk rows preserve cooldowns but discovery previously required callers to read
every page and filter them. Add optional same-row `cooldown_filter` to the existing
search route; preserve the public Rust inventory search wrapper and omitted/null
MCP response shape.

Accept a nonempty set of captured state, inclusive minimum charges and inclusive
maximum remaining seconds predicates. Reject unknown fields and invalid u32s.
Reuse review state classification: charges take precedence over timers. State is
an explicit metadata category, including unknown, so state equality is decidable.
Missing numeric fields instead yield unknown; a known false predicate dominates.
Only true rows survive. This distinction permits diagnostic unknown-state search
without pretending a missing numeric observation is zero.

Apply cooldowns after exact ID/output/reagent/status narrowing, before recipe-view
and material predicates and numeric paging. Expose nonmatching/unevaluable counters
for that stage, null for absent bulk capture and zero for an observed empty list.
Project captured state only when filtering. Preserve original cooldown timestamp;
selected schematics and a newer compatible stock snapshot cannot fill gaps.

No addon/schema/permission expansion, external lookup, game operation, elapsed
recharge or current craftability claim. Partial profession coverage remains partial.
