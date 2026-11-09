# ADR 031: explicit weighted and worst-encounter gear objectives

Date: 9 November 2026. Status: source/offline; no deployment.

A weighted average can favour a gear set with a large encounter-specific loss.
Allow users to choose between weighted gain and maximising the minimum gain
across supplied encounters, without repeating simulations for that choice.

Add `--ranking-objective` to the saved-journal and unified owned-gear CLIs.
Keep `weighted_gain` as default, preserving the weighted-score/hash ordering.
`worst_encounter_gain` sorts descending by minimum per-encounter percentage DPS
gain against that encounter's captured baseline, then weighted gain, then hash.
Every supplied encounter participates; positive weights only break minimum-gain
ties. Return min/max gain and objective score on each retained gear result.

Use the chosen objective during bounded top retention, not merely when sorting
an already weighted-pruned shortlist. Require complete selected results for all
supplied encounters before ranking each page. Incomplete journal traversal
remains completed-pages-only, with existing restart and completeness flags.
Preserve all snapshot/source/native/model guards and saved-loadout provenance.

Validate the objective before journal/collection I/O. It is an aggregation choice,
not a simulation request input, so it must not change raw simulation cache keys
or bound collection scope. Cached runs can be reranked, with the ordinary
current MCP preparation and engine checks still required. Keep point-estimate,
ownership/model/native and full BiS limitations explicit. This is not a
worst-case guarantee for unsupplied encounters or statistical noise.
