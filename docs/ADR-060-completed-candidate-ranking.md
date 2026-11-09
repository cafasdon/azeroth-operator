# ADR-060: Completed candidates from interrupted encounter pages

Date: 2026-11-09. Status: accepted for offline source integration.

## Problem and decision

A bounded page can contain candidates fully simulated across all requested
encounters while other candidates remain incomplete. Previously the final
journal runner discarded every ranking from that page. Retain useful tested
candidates without changing the requested full-search completion criterion.

For incomplete encounter work, add nullable `completed_candidate_ranking`.
It requires every requested encounter result and an equipped baseline (review1)
in each. The review-ID intersection is the only eligible set. DPS additionally
requires positive baseline DPS everywhere. Missing encounters/baselines or
nonpositive DPS baseline yield no partial ranking. Other role metrics retain
valid zero baselines and their existing directions/native units.

Use existing weighted functions with the exact complete subset and original
normalised weights. Do not impute missing values or redistribute weights. The
selected objective is weighted gain or minimum gain across encounters; ties
use weighted gain then gear digest, matching final journal ordering. Standalone
encounter CLI adds --ranking-objective for this partial view; existing full-page
weighted output retains its previous semantics. Upper workflows already carry
the objective. At most 20 rows (or the existing smaller top limit) are retained.
Counts include baseline and distinguish requested, completed, omitted and
truncated results. Omitted means no complete result intersection, not bad gear.
All full-request/global/BiS/feasible/aggregate-uncertainty proof flags stay false.
A baseline-only intersection is valid and suggests no tested alternative.

## Final workflow integration

The saved-journal runner consumes this subset on its first incomplete page,
retains checked source selections/equipment and encounter evidence, and then
stops with incomplete_page. It neither increments completed-window count for
that page nor advances to a later page. Existing complete-page leaders can be
merged using the same objective/tie order. Top-N per page is sufficient for the
same global top-N objective; it does not establish a full-search optimum.

The partial report scope becomes
completed_candidates_across_all_requested_encounters, with nullable
incomplete_page_completed_candidate_count (including baseline, before top-N).
Existing scope values remain for complete results or no usable partial subset.
The unified evaluator retains this in its simulation result while its search,
resolution and simulation completion tests stay unchanged. Restart still
rechecks from the beginning with checkpoints. Full pages use their old ranking.

## Boundaries

No additional engine runs, budget increases, early elimination, cache changes,
character defaults, game actions or deployment. Partial results are point
estimates over the requested encounter profiles, not selection-adjusted
statistical evidence, full feasible-set coverage or complete owned BiS.
No private samples, paths or row metadata are added to the partial projection.
Existing total payload/deadline limits still apply.
[Offline evidence](completed-candidate-ranking-verification-2026-11-09.md).
