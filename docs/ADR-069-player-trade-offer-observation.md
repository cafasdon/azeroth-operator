# ADR-069: Passive player trade offer observation

Date: 2026-11-09. Status: offline source integration.

Add Forever-only `player_trade_offer` to manual saved exports and strict readers.
The section covers six normal outgoing item slots from an already shown TradeFrame.
Each slot contains a stable positive-u32 item ID and quantity, or an explicit
unreadable_or_empty status. Unknown never proves empty. Hidden/unavailable panels
omit the section. Two complete numeric projections and final surface checks guard
against changes during capture; unavailable rows do not erase independent rows.

The source reference is pinned [Forever TradeFrame.lua](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/TradeFrame.lua):
normal slots are 1–6, with slot 7 reserved for enchantment; player item info returns
quantity third and item ID eighth. This is reference API usage, not native client
availability or Blizzard approval. No new policy authorisation is claimed.

Never read the target offer, identity, money, acceptance, enchant slot, links or
free text. Do not open, change, cancel, accept or otherwise operate the panel.
The MCP can inspect the historical section and summarise numeric item quantities.
No trade entry contributes to inventory, recipe materials or gear candidates.
Historical offers do not prove ownership, current location, completed transfer,
current offer or eligibility. Owner-local snapshot controls remain unchanged.

Native behaviour, trade rules, full offer coverage and game testing stay deferred.
Player-only normal item inspection is the implemented scope; other trade aspects
remain unsupported rather than being guessed from this partial observation.

Set `player_trade_offer_changes` to `true` on `compare_snapshots` to compare
reported quantities by base item ID across two captures. The same-character,
same-client and chronological checks apply. If a capture has unreadable or
empty slots, an item with no readable quantity remains unknown. Quantity deltas
require six readable item slots on each side. A delta describes a difference
between saved observations. Trade identity and completion remain unverified.

The offline build and three scoped MCP tests passed. Independent review covered
49 combinations of readable slot counts and exercised identity, chronology,
missing data and false verification flags. Jev 1.14.0 advisory
`a3a965dfce24b8171309eba778b9445d731fa64c8fc056b1f2ef34e192d70d38`
was inconclusive at .34 confidence before the independent probes. It used 414
input and 53 output tokens in .292 seconds.
