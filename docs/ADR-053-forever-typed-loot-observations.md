# ADR-053: Typed non-item loot observations

Date: 2026-11-09. Status: offline source integration; live testing deferred.

## Gap and source

The existing visible loot collector preserves readable item IDs and quantities,
but groups all other rows as unreadable/non-item. The pinned native
[LootFrame source](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/LootFrame.lua)
selects item/currency/money widgets using GetLootSlotType and Enum.LootSlotType.
It reads quantity and currency ID as the third and fourth GetLootSlotInfo values,
then passes currency rows through CurrencyContainerUtil before display.

Therefore a raw API currency quantity is not proof of a final reward quantity,
and money-slot classification is not proof of a copper amount. The source is an
API usage reference, not evidence of native availability or Blizzard approval.

## Contract

Keep `loot_window` coverage `visible_loot_window_slots`, at most 64 contiguous
rows, and existing item/unknown shapes. Add exact mutually exclusive row shapes:

```json
{"status":"reported_currency","reported_currency_id":10,"reported_quantity":3}
{"status":"money"}
```

Currency IDs are positive u32; reported quantities are u32 including zero.
Money has no amount, quantity, name or link field. Private strings, source
entities/GUIDs, tooltips, reward conversions and unrelated API returns are
discarded. Unknown/unstable/restricted rows remain `unreadable_or_non_item`.

Only an already shown Forever loot panel is observed during a manual export.
Use public enum constants rather than hardcoded slot-type numbers. Capture and
recheck the API surface/enum identities and selected values; two stable row
observations and whole-window context checks are still required. Optional type
metadata that is entirely absent preserves the old item path; partial/invalid
or changed typed evidence must not be guessed into an item/currency/money row.
Money needs no slot-info or hyperlink call because no amount is interpreted.

## MCP interpretation

Strict Rust and Python readers accept only these shapes on Forever captures.
Exact inspection exposes raw rows. Existing summary fields and old captures
retain their semantics. When typed non-item rows exist, add
`typed_non_item_observation`: reported-currency/money/unreadable counts and raw
per-reported-currency quantity totals, with basis
`raw_slot_info_before_container_conversion` and false reward/amount/balance
verification flags. These sums can use u64 because 64 u32-max rows fit.

The old `unreadable_or_non_item_slots` count still includes all non-item rows.
Item requirement reviews additionally distinguish known non-item rows from
unreadable rows, so `additional_matching_quantities_unknown` depends only on
unreadable slots. A known money/currency slot does not become a possibly matching
item row. Neither raw currencies nor money slots reduce item shortages.

No displayed loot contributes to inventory, recipe reagents, owned currency
balances or spendable money. Lootability, reward transformation and current
availability stay unverified. No LootSlot, confirmation, rolling, UI opening,
event registration, movement or protected-action bypass is introduced.

## Remaining proof

Native availability, complete loot semantics, container reward identities and
quantities, money amounts, source attribution and pickup rights remain open.
Source packaging/fixtures are distinct from installation and live acceptance.
See [dated verification](forever-typed-loot-verification-2026-11-09.md).
