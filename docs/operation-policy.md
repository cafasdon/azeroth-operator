# Operation scope and evidence

Reviewed 2026-11-03. This records project decisions and evidence limits, not Blizzard approval. Technical API availability alone does not establish permission. Unknown permissions remain disabled. No live client has been qualified.

## Published sources

- [Blizzard UI Add-On Development Policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534), official Kaivax opening post, read directly. Addons must be free with visible code, avoid detrimental effects, advertising and in-game donation requests, and comply with the EULA. Blizzard can disable functionality. Community replies are not policy evidence.
- [Blizzard EULA](https://www.blizzard.com/en-us/legal/08b946df-660a-40e4-a072-1fbde65173b1/blizzard-end-user-license-agreement), official indexed search text retrieved; direct page fetch returned HTTP403. The retrieved clauses prohibit unauthorised automated game control and unauthorised data collection, with third-party UI exceptions at Blizzard's discretion. Full current regional/account terms were not verified. Do not describe this as a complete legal review or an express authorisation for this project.

The engineering interpretation is to use the ordinary addon interface for player-requested exports and analyse saved observations outside the game. This is not a blanket exemption for external read tools, remote control or any callable addon API. No policy source reviewed specifically approves this MCP bridge.

## Operation matrix

These statuses describe Azeroth Operator, not every possible third-party addon. Shared prohibitions apply to Retail, each Classic variant and Forever independently.

| Operation | Retail | Classic | Forever | Current boundary/evidence |
| --- | --- | --- | --- | --- |
| Inspect installation metadata and addon hashes | Implemented | Metadata only, explicit variant | Implemented | Read-only explicit paths; no live API qualification |
| Capture equipment, carried bags and stats | Development addon | Fixture only | Fixture only | Player invokes `/aoexport`; combat refused; API failures unavailable |
| Capture traits, professions, quest log, visible currencies, achievement points, macros, bindings and action bars | Fixture verified | Per-API availability only | Per-API availability only | Coverage varies; currency/achievement data partial; no family fallback |
| Persist export | Player reload/logout | Same boundary | Same boundary | Companion never triggers either action |
| Transfer and inspect saved snapshots | Implemented | Schema supported | Implemented transfer configuration | Strict data parser and authenticated owner store; no memory/packet reads |
| Compare observations and review equipment selections | Implemented | Schema supported | Schema supported | Draft JSON only; no gear legality, optimality or live ownership guarantee |
| Return original captured talent import string | Development support | Disabled | Disabled | Retail only; staged changes refused; player reviews/imports manually; game acceptance unverified |
| Generate optimised builds or new game import formats | Not implemented | Not implemented | Not implemented | Needs client-specific metadata, format verification and policy evidence |
| Equip gear, switch talents or apply action bars/macros/bindings | Disabled | Disabled | Disabled | No MCP execution path; a player click is not assumed to authorise arbitrary queued automation |
| Sort/move/split items, bank/mail transfers, sell/delete/disenchant | Disabled | Disabled | Disabled | No inventory mutation or destructive operation exposed |
| Auction bids/listings, purchases, crafting, trade or chat sends | Disabled | Disabled | Disabled | No transaction or messaging tools |
| Graphics/CVar configuration proposals | Not implemented | Not implemented | Not implemented | Requires exact client settings, hardware evidence and bounded file-change workflow |
| Combat, movement, targeting, quests, queues, farming, anti-AFK | Excluded | Excluded | Excluded | User's rules-only scope; no autonomous gameplay or input automation |
| Memory reading, injection, packet interception, unlockers, protected-action bypass | Excluded | Excluded | Excluded | User prohibition and unauthorised-software concerns; never a fallback |

## Requirements for any future enabled action

Record the exact operation, client family/variant/build/interface, official policy/API evidence, player interaction required, combat/protected/secret-value restrictions, data prerequisites and observed result. Unknown evidence must remain explicit. A working fixture or successful API call is not enough to enable execution. Preserve original scope: supporting a review draft does not fulfil an equip/build/item-management request.

Actual enforcement today is structural: seven read-only MCP tools, a reader container with a read-only owner store, a separate bounded data importer, no addon external-command intake and no game mutation functions. Inspect `src/main.rs`, `addon/Core.lua`, `addon/Collectors.lua`, `tools/gateway_profile.py` and the deployed profile when changing this boundary. Annotations and this document alone are not enforcement.

No game action is newly enabled by this record. Live qualification, operation-specific policy evidence and feature implementation remain outstanding in ROADMAP.md.
