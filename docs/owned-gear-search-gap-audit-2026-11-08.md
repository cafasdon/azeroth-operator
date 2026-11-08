# Owned-gear search gap audit — 8 November 2026

Baseline `7d81ea0a9ff2ef3caef18d51df144aaa97b95225`.
This records current source and offline behavioural evidence against the user's
full owned-gear optimisation requirement. It adds no runtime capability and
does not qualify the installed MCP, game client or a best-in-slot result.
Live testing, installation and deployment remain deferred.

## Current search boundaries

`tools/rank_forever_slot.py` enumerates requested slot choices, including keeping
the captured baseline. It rejects more than 1,024 combinations. The alternative
draft budget defaults to 64 and accepts at most 128. Without collapse it bounds
proposed alternatives; with collapse it bounds distinct resolved alternatives.
The collection phase has a 45-second deadline. These are real rejection limits,
not an ability to resume the remaining full search.

Existing collapse handles identical physical copies and equivalent resolved
simulator inputs while retaining physical-source constraints. It does not make
distinct gear alternatives disappear. Single-swap and maximum-swap restrictions
change the search space; their winners do not establish a full-set optimum.
The report preserves restrictions and inventory evidence, and always keeps
`best_in_slot_verified: false`.

Simulation also retains the captured level-60 qualification guard. Planning for
other/unknown levels is separate from simulation. Larger enumeration alone does
not qualify a broader engine model, complete inventory or native equip legality.

## Independent author probes through the real local MCP

Synthetic owner-local stores used the existing pinned catalogue and actual
stdio executable. No simulator, real character data or game client was used.
Each collection was bounded by 45 seconds; all probes completed normally.

An initial identical-copy probe used 31 and 32 physical copies of catalogue ring
4549. With collapse enabled, both returned nine combinations and three resolved
drafts including baseline. This disproved the initial assumption that increasing
identical-copy count would expose the combination limit; existing collapse is
effective for that case.

A second probe selected distinct catalogue ring IDs with no recorded suffix,
class allowlist or profession requirement. The fixture represented captured
base links, not native equip qualification. Two requested slots were `finger1`
and `finger2`, collapse was enabled and the alternative budget was 128.

| Distinct captured items | Swap restriction | Observed result |
| --- | --- | --- |
| 31 | None | `GearSearchLimit`: resolved search exceeds the 128-alternative budget. |
| 32 | None | `GearSearchLimit`: search exceeds 1,024 combinations. |
| 32 | One | Admitted: 65 combinations, 33 resolved drafts including baseline. |

The unpruned products are 32² = 1,024 and 33² = 1,089 for the first two cases.
The 31-item case reaches the separate draft limit rather than finishing that
product. The successful restricted run is evidence for the restricted space
only. No partial failure result was treated as a ranking or an optimum.

For scale, five choices in each of 17 slots have an unpruned product of
762,939,453,125. That arithmetic is not a count of legal equipment sets or an
estimate of runtime. Physical-copy constraints, legality and provable equivalence
may remove combinations, but no current evidence bounds all such reductions.

## Remaining requirement and evidence gap

The current implementation cannot establish full owned best-in-slot merely by
raising a flag or relaxing one limit. The missing outcome includes complete
qualified inventory, exact item/effect resolution, legal full-set coverage,
supported character/encounter models and measured ranking uncertainty.

For search execution specifically, the missing evidence is an account of all
feasible requested combinations beyond a single bounded in-memory request:
evaluated, excluded with a valid reason, proven equivalent, or left unresolved.
An interrupted or budget-exhausted search is incomplete. A shortlist or a
single-swap substitute does not close that gap. Chunking alone would also leave
candidate/model completeness and uncertainty unproven.

This is the next substantive gear-search gap identified by the audit. It does
not remove the active-character requirement, deferred native qualification or
the broader system coverage objective.

## Regression evidence

With `PYTHONPATH=.;tests`, direct command
`python -m unittest test_gear_draft_dedup test_rank_inventory_evidence -q`
passed three tests, 8.180 seconds. These exercise real MCP collection, physical
copy constraints, equivalent-input collapse, bounded rejection, saved inventory
evidence, changed/legacy evidence handling and plan-only CLI reporting.
They do not execute the simulator or prove full-set coverage.

No runtime source, policy, guard or limit changed. This dated evidence record
and its coverage-audit link are the only repository changes.
