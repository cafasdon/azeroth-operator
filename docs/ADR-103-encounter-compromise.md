# Saved encounter compromise gear

The saved-journal and owned-gear evaluator CLIs accept
`--include-encounter-compromise`, with optional `--encounter-compromise-offset`.
The report ranks eligible saved gear by minimax regret: the smallest maximum gap
from each encounter's best eligible saved improvement. It adds a compromise view
beside the weighted ranking, worst baseline improvement, leaders and Pareto view.
Existing ranking objectives and default output remain unchanged.

For each encounter j, let b[j] be the maximum role-adjusted improvement across
all eligible completed saved candidates. Candidate i has gap b[j] - gain[i,j],
and worst regret max_j(gap). Smaller worst regret ranks first; gear SHA-256 breaks
ties. Example: gains [20,-10], [10,30], [0,0] give reference best [20,30] and
worst regrets 40, 10, 30. Thus [10,30] is the compromise. Gaps use the existing
improvement unit, not percentage differences relative to the best raw metric.
Encounter weights do not enter this objective. The existing per-encounter floor
filters both the candidates and reference-best population before analysis.

All eligible candidates are retained, including dominated candidates: dominance
can leave a tied minimax score. Distinct tied gear sets remain distinct. Exact
fractions of the saved integers/floats control comparisons and output as decimal
numerator/denominator strings. This preserves adjacent large integers and avoids
overflow when finite extreme values have a gap larger than float range. It does
not repair uncertainty or rounding already present in upstream simulation gains.

The analysis stores at most 4,096 distinct candidates, supports one to eight
encounters and shares the existing run deadline. Reference maxima and gaps take
linear work in candidate count times encounter count, then a score sort. No extra
engine or MCP requests occur for this analysis. It uses no private iteration
samples and writes no new persistent data. Only requested result pages receive
the existing checked saved-loadout join before final journal readback.

Only a stable, fully visited saved journal with complete encounter results can
produce a known ranking. Incomplete source results, candidate exhaustion or
deadline expiry return no gear sets, counts, best values, regret values or digest.
The final hash phase checks the deadline again. An eligible empty pool has count
zero, no best values and no minimum regret. Offset is 0–4,096 and requires the
option when nonzero; page size is the existing `--top` bound of 1–20. The digest
binds all eligible saved gear keys and exact improvement vectors, independent of
page, insertion order and equivalent integer/float representation.

This is a point-estimate compromise within saved requests, not statistical
superiority, complete feasible-set coverage or unconditional owned best-in-slot.
It does not establish fresh active identity, equip legality, complete ownership
or accurate native encounter behaviour. No game action, deployment or live test
is introduced.
