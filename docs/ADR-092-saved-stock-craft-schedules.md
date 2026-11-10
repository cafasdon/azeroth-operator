# Saved-stock resource-order witnesses

Date: 10 November 2026. Status: source/offline only.

Add optional stock_schedule to plan_forever_recipe_restock for single or joint
targets. Omission/null preserves existing results. Reuse the fixed planned
counts and existing selected/bulk conflict, ownership, yield and count guards.
This does not choose alternative recipes, larger counts, bank withdrawals or
new acquisitions. Initial stock comes only from the same capture's bags.

Search individual-craft orders with an iterative depth-first traversal and
backtracking. A craft consumes its fixed inputs before adding minimum base
output. A state is the remaining-count vector: at fixed minimum output, its
stock ledger is determined by completed counts regardless of path. Memoise
these states. Candidate recipe order is deterministic by ID. No recursion is
used for up to 8,000 crafts. State budget includes the initial state, defaults
to 10,000 and accepts 1..50,000. Budget exhaustion is unknown, not infeasibility.

Return witness_found only after replay of the complete path checks every input,
planned count and final target. If all reachable states for those fixed counts
are exhausted, return no_schedule_for_planned_counts. This is only the captured-
bag/minimum-output model; alternative counts, yields, sources, seeds or native
behaviour can differ. An exhausted budget returns no partial resource-order steps.
Neither failure status invents a current inventory fact or says crafting is
impossible in the game.

A complete witness coalesces adjacent repeats of the same recipe and supports
offset/limit pages (limit 1..50, default 20). A step means repeated individual
crafts with output available after each, not one native batch command. Digest
binds the complete ordered witness, numeric recipe model and relevant initial
bag counts; it remains the same across pages. Root snapshot_id still identifies
the owner-local capture. Failed/unknown searches return no steps or digest.

This finer model can resolve resource order in seeded cycles and can credit
output within repeated crafts of one recipe. Existing whole-recipe gross and
ordered material ledgers remain unchanged and have their scope stated when
the optional witness is requested. Their conservative shortfall can therefore
coexist with a per-craft witness. Static dependency_order_status can remain
cyclic while the finer stock_schedule supplies an interleaved resource witness.

MCP options must be an object or null. Reject arrays, unknown fields, invalid
numeric types and bounds without echoing values. Search has no network, game
action, public indexing or snapshot writes. Native eligibility, variants,
cooldowns, charges, conditions, bag space, current inventory and order optimality
remain unverified. A resource witness is not a command or native craftability
claim. Installation, deployment and live acceptance remain deferred.
