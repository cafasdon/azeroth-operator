# Equipment review contract

Executor checkpoint: source-only `EquipmentOperations.lua` performs one Retail carried-bag-to-equipment swap. It requires exact source/target GUIDs, explicit item ID, carried-bag limits, unlocked slots, an empty cursor and no combat/casting. Cursor compatibility is checked after pickup, then target GUID and the displaced item's return are read back. Unexpected state stops without clearing the cursor, retrying or guessing an undo. Tests cover the full three-call swap, stale GUIDs, occupied cursor, two-hand refusal and incompatible-slot failure after pickup. No live equipment has changed; compiler, review/apply receipts, richer multi-slot handling and deployment remain unfinished. Two-handed items are explicitly refused until off-hand side effects and bag destinations are represented in the operation.

Operational groundwork (source only): Retail equipment and carried-bag captures can now include an optional item `guid`, obtained with C_Item.GetItemGUID and Blizzard's [ItemLocation constructors](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_ObjectAPI/Mainline/ItemLocation.lua). Missing, failing or secret GUIDs remain absent, so current readers still support older observations. Lua producer tests preserve separate bag/equipment instance identities and omit unavailable GUIDs; Rust bounds and validates the optional field. Full source verification passed. This schema extension is not deployed and does not yet equip items. The future executor must bind GUID plus exact source/target locations and account for the cursor and displaced item, following the ordinary [equipment manager flow](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_FrameXML/Mainline/EquipmentManager.lua).

`review_equipment_selection` returns a JSON document for a player to inspect.
It is an analysis operation. It has no game command, macro, addon import or
execution path. All three client families use the same snapshot contract;
fixture tests do not establish compatibility with any live game build.

Example MCP arguments (replace the hash and locations with an inspected snapshot):

```json
{
  "snapshot_id": "<stored snapshot SHA256>",
  "selections": [
    {
      "target_slot": 11,
      "source": {"kind": "bag", "bag": 0, "slot": 3},
      "expected_item_id": 123
    }
  ]
}
```

Every selected location must exist in that snapshot and contain the expected
item ID. A source location cannot be selected twice. Target slots cannot repeat.
Different locations containing the same item ID remain distinct observations;
this does not prove that both can legally be equipped. Equipment and carried-bag
sources are supported. Forever also accepts observed bank sources, for example
`{"kind":"character_bank","container":6,"slot":1}` or
`{"kind":"account_bank","container":13,"slot":1}`. These reference saved item
locations only. Live access to banked gear and mail remains outside the draft.

A local `review_forever_owned_gear` query with `simulator_slot` includes a
`draft_selection` for each result. Pass one or more of these selections to
`review_equipment_selection` with the same snapshot ID. Each selection
preserves the saved source location and the requested target. The resulting
draft still checks combined restrictions and simulator input resolution.

The response has format `azeroth_operator.equipment_review.v1`, the exact
`snapshot_id`, client/character/capture identity, changes sorted by target slot,
and equipment source slots left vacant by the requested selection. Swaps have
no vacated slots. Selecting an item in its existing slot reports
`already_selected`. All source items are resolved against the original snapshot,
never against a partially changed inventory.

`selected_item_observed` and `previous_item_observed` preserve the available
exported data. A null previous observation is not proof that the live slot is
empty. A displaced item has no assigned destination. The review does not infer
an executable ordering or claim sufficient bag space.

The status remains `draft_unverified_compatibility`, with `game_execution:false`
and `requires_player_review:true`. Compatibility, class/level requirements,
unique-equipped restrictions, weapon pairing, inventory capacity, current
ownership and performance are explicitly unknown. Snapshot schema 1 has no
item GUID, account or region identity. The player must check current items and
requirements in the appropriate client before making changes manually.

Local Forever reviews with a configured catalogue add a separate
`catalogue_two_hand_pairing` check. It flags the projected combination of a
two-handed main-hand item and an occupied off-hand. The general weapon-pairing
check remains unknown.

The same catalogue adds `catalogue_slot_fit` for each requested target. A
match means the pinned simulator lists that slot for the base item. Unknown
items and slots outside the simulator, such as shirts and tabards, report
`unknown`. The entire slot result is null if catalogue verification fails.
Character eligibility and full slot legality remain unverified.

`catalogue_unique_item_duplicates` lists repeated base item IDs marked unique
in the projected equipment, with their target slots.
`catalogue_shared_limit_categories` lists nonzero categories shared by two or
more projected slots, with the associated item IDs. It does not resolve
category caps or restrictions for uncatalogued items. A null result means the
catalogue could not be verified.

`simulator_equipment_spec` contains 17 positional item inputs in pinned
WoWSims order when all projected observations in those slots resolve. Empty
slots in the capture projection use empty objects. Shirt and tabard are
outside the simulator model, and explicitly targeting either withholds the payload.
Unresolved or misplaced items appear in `simulator_equipment_unresolved_slots`.
The payload is withheld for projected two-handed conflicts, duplicate unique
items and shared limit categories. It is also withheld when the engine would
move a lone item from the second ring or trinket slot, or move a flexible
off-hand weapon to an empty main hand. Character eligibility, encounter
configuration and actual equip legality require separate evidence.

For a captured Forever baseline, call `review_equipment_selection` with its
snapshot ID and `"selections":[]` on the local catalogue-enabled MCP. The review
lists no requested changes or vacated slots and applies the same simulator
resolution checks to the captured equipment.

An equipment optimiser and legal game import format remain separate, unfinished
features. They require richer item metadata, operation-specific policy evidence
and live client verification; this draft must not be presented as their substitute.
