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
