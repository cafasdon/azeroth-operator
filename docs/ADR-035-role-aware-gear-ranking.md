# ADR 035: carry role metrics through saved gear evaluation

Date: 9 November 2026. Status: source/offline; no deployment.

The saved-window executor already supports DPS, DTPS, TPS, death, HPS and OOM.
The upper encounter runner overwrites the choice with DPS, and journal ranking
assumes DPS fields. Carry an explicit validated `--rank-by` through encounter,
page, saved-journal and unified entrypoints, defaulting to DPS before any I/O.

Keep existing DPS percentage ranking, output fields, positive-baseline behaviour
and sensitivity logic. For other metrics, use a bounded numeric role aggregator:
TPS/HPS improvement is candidate minus baseline, DTPS/OOM is baseline minus
candidate, and death improvement is 100 times the reduction in simulated death
fraction. These use rate units, seconds and percentage points respectively.
Zero baselines remain meaningful without a ratio denominator. Expose explicit
metric/unit metadata and generic numeric improvement/value arrays; do not label
these values as DPS percentages.

Require one to eight complete matching review sets, unique positive integer review
IDs, matching baseline IDs, finite domain-valid values and normalised positive
weights. Reject missing fields and overflow. Project numeric data only and leave
input objects unchanged. Repeated encounter profiles remain valid.

Retain both weighted-average and minimum-encounter-improvement objectives; apply
the chosen metric/objective before top pruning across pages. Bind selected-metric
baseline values across pages alongside existing context fingerprints. Missing
work remains incomplete. Keep source scope/loadout, model, owner and engine
guards and budget/finalisation rules intact.

Use existing tank/healing flags and cache families. Switching metrics within a
family reuses its validated summaries; crossing families requires the appropriate
summary capability. No change to raw simulation requests or collection scope is
needed for an aggregation choice. Tank evaluation still requires explicit tank
assignment. Templates still require explicit valid rotations; upstream healer
stats-only presets are not a supplied healing model. This implementation does
not qualify all classes, combat accuracy, native eligibility or full owned BiS.
