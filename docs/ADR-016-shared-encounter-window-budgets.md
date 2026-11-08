# ADR 016: Shared budgets for saved-window encounter comparisons

Date: 9 November 2026. Status: source/offline implementation.

The saved-window executor previously handled one encounter. A separate CLI now
visits up to eight explicit DPS encounter profiles with one shared new-run,
new-iteration and work-deadline budget. It uses the same fresh MCP preparation,
compatible-window grouping and private checkpoint path, not prepared JSON. It
does not multiply a budget per profile.

Checkpoint-only mode revalidates inputs and reads numeric records without new
engine work. The wrapper switches to this mode when shared run/iteration budgets
are exhausted, allowing later cached profiles to remain visible. Every child
receives the inherited common deadline and five-second finalisation boundary.
The aggregate checks that boundary before processing and after encoding the
report. Unvisited profiles remain explicit. Checked I/O/subprocess deadlines
are not hard real-time guarantees.

Comparability requires one engine, common raid/APL/type after removing gear and
encounter/options, matching non-encounter file fingerprints, identical source
coverage/request counts and consistent global review-to-gear identities. Different
encounters, iterations and seeds are permitted. Changed builds/source contexts
reject rather than silently combine.

Weights must be positive and finite, one per requested profile. Scaled
normalisation avoids overflow; underflow that would erase a requested profile
rejects. Weighted values are relative DPS gains against each encounter's own
baseline. Duplicate profiles keep their weights and reuse matching checkpoints.

Weighted ranking and gain sensitivity require complete results for every selected
resolved request in every profile. Missing profiles are not zero gains or losses.
Nonpositive baseline DPS leaves normalised ranking unavailable while preserving
per-profile observations. Sensitivity describes point estimates, not statistical
certainty or best-in-slot. Individual tank/healing saved-window rankings remain
available separately; aggregate weighting currently covers DPS.

Output is bounded to 16 MiB and excludes full requests/paths. Existing engine,
model, privacy and coverage boundaries remain. Native qualification, complete
ownership/feasibility/models, all-window orchestration and unconditional BiS
remain separate requirements.
