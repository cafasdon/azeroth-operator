# ADR 019: bounded captured reagent-chain drafts

Date: 9 November 2026. Status: source/offline implementation.

The existing recipe review accepts explicit craft batches and can suggest a
dependency order. It does not discover intermediate producers or determine
their counts. Add an opt-in `suggest_reagent_chain` mode for one exact bulk
recipe, using captured fixed material rows and reported minimum output ranges.

Build an output-item index once. Recursively expand an uncovered reagent only
when it has exactly one captured fixed producer. Reuse the same modeled
inventory across all steps, consume requirements before adding conditional
outputs, retain leftovers, and use ceiling division by the reported minimum
yield. Existing exact recipe planning validates every selected producer's
observation/learning consistency against the same compatible inventory capture.
Repeated producer steps are permitted when later branches need additional units.

No captured fixed producer yields an external material requirement, not a claim
that no producer exists in game. More than one candidate blocks rather than
choosing by ID, order or guessed cost. Uncovered demand that re-enters an active
recipe blocks as cyclic. Unknown output/minimum yield, unsupported materials,
conflicting producer learning, completely unobserved required inventory, more than eight
steps or more than 1,000 crafts for any step also block. Ancestors reserve step
budget during recursion. Blocked results may retain completed prefix steps but
do not expose complete external requirements or a final stock projection.
Ordinary root recipe/request/inventory validation precedes expansion; those
failures, including root learning conflicts, retain the existing whole-tool error.

Leaf shortages are conditionally acquired in the model to continue deriving
the chain. A complete draft can require external materials and never establishes
that the player owns them. Inventory is only the observed bags/banks lower
bound; missing sections remain explicit. Free fixed requirements do not need an
inventory observation. Output is conditional on completion using reported
minimum yields, which are not independently verified. Nonmaterial checks remain
captured or unknown; skill gains, cooldown availability and craftability are
not inferred from material feasibility.

This is deterministic bounded expansion, not a global optimisation or proof of
the best crafting route. Explicit exact ID and standalone recipe mode are required;
batch and dependency-order modes cannot be combined with this option. Their
existing behaviour is preserved. Later ADRs below add explicit producer choices
and historical external-leaf merchant quotes. All gameplay
execution and verified ownership/craftability/optimisation flags remain false.

Subsequent extension, 9 November 2026: ADR 020 implements caller-selected
captured producers for ambiguity. Automatic selection remains unambiguous-only.
ADR 021 subsequently allows captured merchant quotes for complete external
material needs, superseding the initial merchant-mode prohibition.
