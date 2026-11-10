# ADR-077: Passive owner auction cache observations

## Intended behaviour

Manual Forever exports may observe the already-loaded owned-auction cache
when AuctionHouseFrame is already shown. The collector reads only
C_AuctionHouse.GetNumOwnedAuctions and GetOwnedAuctionInfo. It does not load
UI modules, request updates, query the market or invoke auction transactions.
No background polling or external game-command path is introduced.

The new owned_auctions section has coverage loaded_player_owned_auction_cache
and a contiguous slot map of zero to 256 entries. Each contains positive
auction_id (at most 2^53-1), positive uint32 item_id and quantity, and raw uint32
reported_status. Duplicate auction IDs reject. Status is retained without
interpreting sale, expiry, delivery or present availability. Two matching
numeric projections, stable counts and stable panel/API context are required.
Hidden, unavailable, malformed, restricted, over-budget or changing data is
reported unavailable, never an empty list. An explicit zero cache count yields
an empty observed map; it does not prove that the player has no auctions.

Raw API structs are projected field by field. Bidder names, item links, bids,
prices, time values and arbitrary extra fields are neither read nor exported.
Both snapshot readers reject unknown section and row fields and enforce the
same numeric, identity and slot constraints. The existing owner-bound store,
Forever-family restriction and MCP inspection/coverage discovery apply.
Auction rows confer no carried/bank ownership, inventory stock or wallet credit.
They are not included in the public knowledge index. Current cache ownership,
character binding and native API behaviour still require native qualification.

## Numeric transport

Lua 5.1 tostring may round large integer IDs or emit exponent notation. Core
now emits integral values in the exact JSON-safe range with fixed zero-decimal
formatting; finite fractional values keep the existing representation. The
safe range is a transport boundary, not an auction-house limit. Existing
small-integer exports retain their numeric meaning. Auction schema rejects
values beyond this safe range instead of rounding them or decoding money.

## Reference and rollout boundary

The checked Forever API declarations describe the two cache getters and the
OwnedAuctionInfo fields used here. The UI source defines AuctionHouseFrame;
source availability does not establish its presence on an installed client.

- [API declarations](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/AuctionHouseDocumentation.lua)
- [Frame definition](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_AuctionHouseUI/Shared/Blizzard_AuctionHouseFrame.xml)
- [Owned-auction UI](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_AuctionHouseUI/Shared/Blizzard_AuctionHouseAuctionsFrame.lua)

Checked 10 November 2026. These are mirrored Blizzard UI/API source files, not
policy approval. Existing project policy boundaries remain in force. Update
both readers before installing the exporter. No installation, deployment or
live game testing is part of this change.

Saved-cache discovery and inspection (10 November): list_snapshots accepts
owned_auction with one or more exact item_id, auction_id or reported_status
predicates. All predicates match the same captured row. Missing/unreadable
sections and observed sections with no match do not satisfy the filter.
Latest-per-character resolution takes place before these predicates, so an
old listing is not resurrected when a newer capture has no matching row.

inspect_snapshot accepts owned_auction_filter containing filter, offset and
limit (default 20, maximum 50) for section owned_auctions. Rows sort by numeric
captured slot before paging. Matching counts and matching_reported_item_units
cover the entire filtered capture, not just the returned page. The latter is
a sum of reported quantity units; it is never inventory or money credit and
may combine several base item IDs. Missing sections yield null rows/counts;
known empty/no-match captures yield zero counts and an empty list.

Filters require at least one non-null predicate, positive uint32 item IDs,
positive JSON-safe auction IDs and raw uint32 status (zero valid). Unknown
fields, malformed types and invalid paging reject through redacting MCP
argument parsing. A listing match includes at most 20 rows, a full match
count/unit total and a truncation flag. No filter preserves existing raw
inspection. These are searches over historical numeric cache observations;
no status meaning, current auction ownership, sale/delivery or native action
is inferred. Existing owner/family boundaries remain.
