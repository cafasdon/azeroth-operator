# First-party data and external gap sources

The preferred source is the owner's ordinary, player-requested addon export.
Local analysis derives records and relationships from exact validated captures.
It does not need a website, community upload, remote inference or a live game
connection. Derived knowledge stays in the same owner's private environment.

## Source order

1. Exact owner-observed data for the requested client/build and capture.
2. Blizzard-published data through an applicable documented interface, where
   the access and use conditions are established for that product.
3. Explicitly licensed external datasets for identified missing facts, with
   their product/build, revision, licence, attribution and update date retained.
4. Unknown. Absence is not zero, an impossible prerequisite or proof that an
   item/recipe cannot exist. Conflicting builds/sources must stay distinct.

The existing pinned QuestieDB world cache and WoWSims catalogue are external
reference inputs, not our own extraction. They remain separate. New local
capture queries must not silently fetch from those projects or any website.
Use of third-party code or parsing schemas is also distinct from permission
to acquire or redistribute the underlying game data.

## Acquisition boundaries

Public accessibility, robots rules, an API's existence, a tool's licence or
installation of a game do not themselves establish permission for bulk reuse.
New third-party acquisitions need an identified data licence or permission,
the intended-use conditions and a bounded supported export/API. A source
without that evidence stays unavailable; do not scrape it as a fallback.
No bypass of login, paywalls, rate limits, encryption, protected values or
access controls is part of this route. Personal observations are not uploaded
to community services automatically and are not a public dataset by default.

## Client archive extraction is not enabled

The WoWSims db2tool documentation establishes technical feasibility of reading
CASC/DB2 files, not Blizzard authorisation for this project. The previous chat
answer omitted this distinction. The indexed official EULA text retrieved on
14 November 2026 restricts unauthorised mining of platform data. The direct
regional EULA page returned HTTP 403; no complete account/region legal review
or express extraction approval was established. Therefore no client archive
extractor, CDN miner or community key download was run or enabled here.

The ordinary addon policy also requires EULA compliance. Using saved addon
observations follows this project's existing bounded capture approach; it is
not a claim that Blizzard approved this MCP or every possible API operation.
The current change operates on exports already captured by the owner and
does not add in-game enumeration or automation.

Sources checked:

- [Blizzard EULA](https://www.blizzard.com/en-us/legal/08b946df-660a-40e4-a072-1fbde65173b1/blizzard-end-user-license-agreement)
- [Blizzard UI Add-On Development Policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534)
- [WoWSims db2tool technical documentation](https://github.com/wowsims/forever/blob/master/tools/db2tool/README.md)

These records define evidence limits. They do not guarantee that legal claims
or account enforcement can never occur.
