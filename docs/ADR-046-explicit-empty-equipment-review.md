# ADR 046: explicit desired empty equipment slots

Date: 9 November 2026. Status: MCP review and offline adapter integration.

Full equipment-set optimisation must be able to represent removing an equipped
item, not only keeping or replacing it. Existing reviews support implicit
off-hand clearing for selected two-handed weapons and vacating moved sources;
there was no general explicit empty-slot proposal.

Add optional `clear_equipment_slots` to primary and additional equipment drafts.
It defaults to empty, preserving prior request behaviour and output. Nonempty
requests require Forever, a verified local catalogue and positional layout.
Permit at most 17 distinct simulator equipment slots, each occupied in the
bound saved observation, disjoint from selected targets. Shirt/tabard and
already unobserved slots are rejected. Clearing is a desired state, never an
assertion that the captured slot was empty.

Project explicit holes before catalogue fit, effect, unique-item, requirement
and two-hand checks. Union explicit holes with the existing automatic off-hand
clear in `simulator_cleared_equipment_slots`. Keep immutable source evidence in
`explicitly_cleared_equipment`, sorted by slot, with previous observed items.
`vacated_equipment_slots` retains its existing meaning of moved source slots;
explicit clears are recorded separately. No item destination, available bag
capacity, native equip eligibility or in-game execution is implied.

The existing simulator adapter consumes the resulting exact-slot specification;
its trusted engine/capability checks remain required for execution. The legacy
automatic-placement layout is refused for explicit clears. Additional drafts
share the snapshot/layout but retain independent clear lists.

This establishes the review representation. Automatic search enumeration and
saved journal ingestion of general explicit clear requests remain open; legacy
off-hand-only journal validation is deliberately unchanged. Neither full legal
set coverage nor owned BiS is established by this addition. No addon package,
game installation, deployment or live qualification occurs.
