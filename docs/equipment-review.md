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
sources are supported. Bank, mail and unobserved items are not inferred.

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

An equipment optimiser and legal game import format remain separate, unfinished
features. They require richer item metadata, operation-specific policy evidence
and live client verification; this draft must not be presented as their substitute.
