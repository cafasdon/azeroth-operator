# ADR 027: one resumable observed-owned gear evaluation workflow

Date: 9 November 2026. Status: source/offline implementation.

Existing collectors and saved-journal runners support large coordinate searches
and resumable simulation, but callers must operate the stages separately. Add
`tools.evaluate_forever_owned_gear` as an offline CLI orchestrator. Use the actual
local MCP's read-only review paths, not native game access or a new command tool.
Require an explicit saved snapshot; active-character identity remains unknown.

Always select all 17 simulator slots, all observed storage, no item/set/level/
swap filters, equivalent-input collapse and retained numeric equipment specs.
Reuse existing candidate, identity, source-location, legality and model guards.
The scope is all reported observed candidates, not proof that all owned gear was
captured or every feasible/native model is supported. Do not relax a guard to
force computational completion.

Collect bounded windows into an explicitly bound owner-local plan journal, then
run the existing stable saved-journal multi-encounter evaluator using private
numeric result checkpoints. Give both stages one total monotonic deadline and
give collection an additional stage ceiling. Existing stage APIs accept optional
inherited deadlines without changing standalone defaults. Preserve the existing
simulation five-second finalisation allowance and new-run/iteration/page limits.

Stdio cancellation can surface as an exception group. Only when the collection
ceiling has expired and total time remains may the orchestrator recover a
committed prefix: recompute current source binding, inspect the journal and
verify binding/snapshot identity before simulation. Per-call attempt/insertion
counts then remain null. Early failures, invalid bindings and explicit task
cancellation reject. If the total budget cannot support validation, stop with
committed work retained. Never renew the total deadline to finish another stage.

Validate numeric options, input JSON, engine checksum and output paths before
collection writes. Plan journal and simulation directory must be distinct;
reject links/reparse points and input collisions with their output namespaces.
Reserve the plan database's main, `-journal`, `-wal` and `-shm` paths, including
resolved/hardlink aliases. Existing journal application/schema/private-key
checks remain; no incompatible state is overwritten or silently migrated.

Report coordinate coverage, unresolved proposed coordinates and all-saved
resolved simulation completion separately. Only their conjunction is
`reported_space_results_complete`. A partial collector, unresolved model or
incomplete simulator remains explicit even if a saved shortlist has results.
Rankings are existing weighted point estimates, not statistical/full-set
optimality. Preserve false ownership, active-character, native feasibility,
global-ranking, BiS and game-execution verification flags.

Resumption uses the same snapshot/template/MCP/software/config binding. Changing
those inputs can require a new journal; old evidence remains intact and readable
by its supported tools. No HTTP, data sync, addon installation, deployment,
automatic equipment change or live qualification is introduced.

Owned-gear evaluation can select an explicit character with `--character-name`
and `--character-realm` instead of `--snapshot-id`. The existing owner-scoped
MCP resolver chooses the newest matching historical Forever capture once;
the evaluation pins that snapshot ID. No match fails before plan writes.
Missing/mixed selectors are rejected; there is no newest-alt or active-player
fallback. A resumed journal stays bound to its original capture and rejects a
changed selection. Resolution consumes the shared budget, capped at 15 seconds.
