# ADR 044: conditional comparison of readable loot and explicit requirements

Date: 9 November 2026. Status: source/offline; no deployment.

Loot capture/summary exposes displayed items but cannot compare them with a
reagent or shopping list. Add opt-in `loot_requirements` to the existing saved
loot inspection route, requiring one to 64 distinct positive u32 item IDs with
positive u64 quantities. Validate even when the loot section is absent.

Aggregate readable slot quantities by base item ID and preserve request order.
Expose numeric matching-slot order, readable quantity, minimum covered demand
and remaining demand if those readable items were acquired. Never borrow from
bags, banks, mail, merchants or other captures. Preserve unreadable/non-item slot
counts and possible additional-quantity unknowns. Missing sections return null;
an observed empty window yields zero readable quantities.

The initial plan included a full-window quantity when all rows were readable.
After an advisory preferring revision, author removed that stronger field: all
results now describe only readable saved rows, never a full-window total or
current availability. False readable coverage is not evidence that unknown rows
or other sources cannot meet the request. Quantities are hypothetical, not stock.

No lootability, ownership, variants, source identity or current game state is
verified. All action/ownership flags remain false. No new capture, schema,
permission or MCP action route. Existing inventory and recipe/gear accounting
remains separate. Native qualification and complete acquisition planning remain
open.
