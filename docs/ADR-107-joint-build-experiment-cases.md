# Joint build experiment cases

Date: 15 November 2026. Decision: bind a complete case before joint experiments.

The requested optimiser must consider interactions across gear, talents,
rotation, consumables and timing. Independent stat or slot rankings cannot
represent every interaction. Public methods and their limits are recorded in
[the source review](recommendation-methods-2026-11-15.md).

The first executable slice is a pure experiment-window planner, not a combat
scorer. A case binds the requested client/build/interface, snapshot reference,
class/spec/level, objective, engine, data, encounter and constraint hashes.
Bindings preserve supplied context; they do not verify that the referenced
character, item, model or constraint file exists or is compatible.

Named axes identify ordered candidate-reference pools. Multiple gear axes can
represent different slots; other axes can represent complete talent builds,
rotation policies, timing plans and consumable sets. The planner computes the
declared Cartesian matrix size and emits deterministic mixed-radix windows. It never
allocates or pretends to evaluate that entire matrix. The last axis changes
fastest. Half-open decimal coordinate ranges support exact partitioning and
resume positions beyond JavaScript's safe integer range.

The search identity binds the format version, complete case and every ordered
axis/value. Candidate identities bind that search and coordinate. A caller can
require the expected search identity when it resumes; changed context rejects.
Terminal coordinates and planned windows are not a work-completion ledger.

Strict schemas and manual checks bound input sizes, axes, references, coordinates
and text result envelopes. The MCP accepts no paths, SQL, engine command or
arbitrary callback. The CLI can read one explicit ordinary JSON file. There is
no persistence, source refresh, simulator invocation or game interaction.

All evidence flags remain false for snapshot identity, active character,
source/model compatibility, legality, constraints, inventory coverage, objective
evaluation, simulation and global optimality. Only plan creation is true. Any
level can be described in a prospective case; existing level-60 simulation guards
are unchanged. This does not add a level-14 or level-22 combat model.

Subsequent slices must resolve candidate references under the installed owner's
authority, validate complete candidate combinations, construct pinned simulator
requests, execute bounded jobs and record results. A joint evaluator must bind
results to this exact case, distinguish unknowns from failures, quantify
uncertainty and preserve accounting for exclusions and unevaluated coordinates.
It must not promote this planner's matrix count into a best-in-slot certificate.

No class mechanics are generated from guide prose and no closed website engine
is copied. The event model and native validation remain separate work. See
[usage and offline evidence](joint-build-search.md).
