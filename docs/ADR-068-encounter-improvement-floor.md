# ADR-068: Encounter improvement floor for owned gear

Date: 2026-11-09. Status: offline source integration.

Weighted optimisation can recommend a set that improves the average but loses
performance in a particular encounter. Add optional
`--minimum-encounter-improvement` to `tools.evaluate_forever_owned_gear` and
`tools.run_forever_saved_journal`. With 0, every requested encounter must have a
nonnegative point-estimate improvement relative to its equipped baseline.

Accept a finite number, inclusive of the boundary; negative values permit that
much loss and positive values require improvement. Units match ranking_score_unit:
DPS percent, DTPS damage/second reduction, TPS threat/second gain, death fraction
percentage-point reduction, HPS healing/second gain, OOM seconds reduction.
No value means the existing unconstrained ranking. Baseline also faces the floor.

Apply the floor before top selection, including interrupted pages whose fully
compared candidate intersection is ranked upstream. Missing encounters never
satisfy the constraint. Preserve the original weights and requested profiles.
Empty eligible results are valid and do not mean simulation was incomplete.

Report the selected threshold and excluded candidate instances across visited
pages. The count includes repeated baselines/candidates and is not distinct gear.
Incomplete candidates are not counted as failures. Top limits still bound output.
The constraint uses point estimates, not confidence bounds or native qualification.
No new engine work, cache identity changes, gameplay execution or BiS proof.
