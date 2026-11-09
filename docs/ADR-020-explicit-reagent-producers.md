# ADR 020: explicit captured reagent producer choices

Date: 9 November 2026. Status: source/offline implementation.

ADR 019's automatic chain expansion blocks when more than one captured fixed
recipe produces an uncovered reagent. Choosing a recipe by ID order or guessed
cost would invent the user's preference. Allow up to eight explicit
`reagent_producer_choices`, each mapping a positive item ID to a positive recipe
ID in the same bulk capture. Reagent-chain mode remains required.

Validate list size, unique item IDs and exact fixed output membership even for
choices that might not be needed. Missing, unsupported and wrong-output recipes
cannot become choices. These invalid requests retain ordinary tool errors.
Learning/material/output consistency, readable positive minimum yield, cycles,
step/craft bounds and inventory context still apply when a producer is used;
an explicit choice never bypasses them or falls back to a different producer.

Only consult a choice when the ledger has uncovered demand. Sufficient observed
or prior modeled intermediate stock does not trigger production. For nonempty
choice lists, report requested mappings and the subset consulted for uncovered
demand. The used subset includes attempts whose subsequent expansion blocks;
it is not a record of completed crafting. Unused valid choices are retained as
requests without implying they were needed or are currently usable.

The no-choice helper signature and default result shape are preserved. Omission,
null and empty lists in chain mode retain automatic unambiguous expansion.
Unresolved ambiguity still blocks. The choice cap is independent of the eight
expanded-step budget, which remains unchanged. Repeated producers, conditional
yields and external requirements keep ADR 019's semantics. No new capture,
catalogue lookup, live operation or gameplay action is introduced.
