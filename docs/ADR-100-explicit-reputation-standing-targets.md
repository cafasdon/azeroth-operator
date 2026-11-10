# Explicit saved reputation standing targets

Date: 10 November 2026. Status: source/offline only.

`inspect_snapshot` on `reputation` accepts `reputation_target_standing` when an
exact `faction_id` and `reputation_points_per_turn_in` are supplied. The target is
a signed int32 counter value, not a reaction/rank number. Null/omission preserves
the previous output. Wrong section, coverage inspection, absent faction/gain,
non-integer or out-of-range target fails before source projection, with generic
parse errors. Existing item/copper cost validation remains unchanged.

The new `reputation_target_plan` is separate from all existing next-rank reports.
Only captured standard reputation with usable existing progress evidence supplies
a counter. Missing/other progression kinds and inconsistent non-capped brackets
remain unknown; renown, friendship and paragon units are not reinterpreted.
Standing shortfall is max(target-current,0), with ceiling division by the
user-supplied positive gain. Already met targets need zero turn-ins.

This is constant-gain arithmetic before caps/restrictions. It does not infer
faction reward rates, repeatability or eligibility across ranks. A captured
capped reaction remains visible; a higher target is still only a theoretical
before-caps estimate, never proof that the target can be reached. The projected
counter can overshoot the requested target or a native cap. No target is treated
as a verified game objective or actual future reward.

When costs are supplied, the same capture's bags and wallet provide resource
counts. Item costs sum over the requested turn-ins with checked multiplication;
bank/mail stock receives no credit. Existing copper/reserve arithmetic is reused,
with target-specific labels. The joint supported count is the minimum requested
count and all known supplied resource capacities. Unknown resources suppress a
positive supported count; a known zero bottleneck or an already met target still
gives exact zero under the model. Known upper bounds and unknown counts remain
separate. No supplied cost model yields no resource plan, rather than free access.

Overflow leaves required quantities/copper unknown with explicit flags. All costs
and gains are assumptions, not linked to a specific observed quest or NPC. Old
bank/pending-mail options still belong to the unchanged next-rank views; the new
target plan uses bags/wallet only. The new projection retains numeric/boolean
evidence and no faction name or private source text. No inventory mutation,
payment, turn-in, external lookup, new MCP route or installed change occurs.
Live tests and deployment remain deferred.
