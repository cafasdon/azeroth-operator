# Operation scope and evidence

Reviewed 2026-11-03. This records project decisions and evidence limits, not Blizzard approval. Technical API availability alone does not establish permission. Retail export/readback is partially live-qualified; macro creation is live-qualified; other player-applied mutations have separate qualification gaps. ADR006 supersedes the original blanket read-only implementation target.

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
| Create/edit/delete/move macro scope | Development implementation installed | Disabled | Disabled | MCP prepares bounded data; player reviews/applies through ordinary addon APIs; creation verified; edit/delete/scope-move remain unqualified |
| Change keybindings | Deployed; no-op verified | Disabled | Disabled | One explicit key and context; actual SetBinding/SaveBindings still unqualified |
| Equip gear | Development implementation deployed | Disabled | Disabled | Single GUID-bound bag-to-equipment swap; exact item GUID export verified; swap unqualified |
| Rename/change icon/delete native equipment sets | Development implementation deployed | Disabled | Disabled | Exact captured set preimage; explicit player review/Apply; no live set mutation |
| Use native equipment sets | Development implementation deployed | Disabled | Disabled | Exact GUID preimages/readback, ignored-slot preservation, explicit empty current-slot preconditions; unresolved target slots refused; player Apply; live unqualified |
| Assign/unassign equipment-set specialisations | Development implementation deployed | Disabled | Disabled | Full catalogue/spec-ID preimages; displaced association shown; one player-applied native call, exact readback; live unqualified |
| Create/save native equipment sets | Development implementation deployed | Disabled | Disabled | Full catalogue/equipped preimages, explicit ignored/empty preconditions, temporary flag restoration, player review/Apply and independent readback; nonignored empty slots refused; live unqualified |
| Switch saved talent builds | Development implementation deployed | Disabled | Disabled | Exact captured build and current-state checks; native player-triggered talent UI path; live switch unqualified |
| Rename/delete saved talent builds | Development implementation deployed | Disabled | Disabled | Full current-spec catalogue and active-content checks, explicit player review/Apply, exact readback; selected deletion requires 0.16.0-dev and verified API/frame/dropdown selection clearing; native unqualified |
| Move/swap captured action slots | Development implementation deployed | Disabled | Disabled | Exact source/target and macro-content preimages; player review/Apply; live unqualified |
| Create saved talent builds / import supplied builds | Development implementation deployed | Disabled | Disabled | Player review/Apply; complete catalogue and active-content checks; explicit shared-bars/hash acknowledgements where applicable; native unqualified |
| Edit individual talent nodes / generate new action-bar entries | Not implemented | Not implemented | Not implemented | Remains in scope; no protected-action bypass |
| Copy captured actions into empty slots | Source executor only | Disabled | Disabled | Spell/item/macro identity and source preservation; MCP transport/review/receipt integration pending; native unqualified |
| Move carried items | Development implementation deployed | Disabled | Disabled | Exact-instance move to explicit empty slot; first live attempt outcome_unknown, no replay; timing fixes fixture-tested |
| Split carried stacks | Development implementation deployed | Disabled | Disabled | Explicit amount, empty destination, source conservation and distinct destination identity; live unqualified |
| Sort/merge/swap items, bank/mail transfers, sell/delete/disenchant | Disabled | Disabled | Disabled | Not yet implemented or exposed |
| Auction bids/listings, purchases, crafting, trade or chat sends | Disabled | Disabled | Disabled | No transaction or messaging tools |
| Graphics/CVar configuration proposals | Not implemented | Not implemented | Not implemented | Requires exact client settings, hardware evidence and bounded file-change workflow |
| Combat, movement, targeting, quests, queues, farming, anti-AFK | Excluded | Excluded | Excluded | User's rules-only scope; no autonomous gameplay or input automation |
| Memory reading, injection, packet interception, unlockers, protected-action bypass | Excluded | Excluded | Excluded | User prohibition and unauthorised-software concerns; never a fallback |

## Requirements for any future enabled action

Record the exact operation, client family/variant/build/interface, official policy/API evidence, player interaction required, combat/protected/secret-value restrictions, data prerequisites and observed result. Unknown evidence must remain explicit. A working fixture or successful API call is not enough to enable execution. Preserve original scope: supporting a review draft does not fulfil an equip/build/item-management request.

Actual enforcement today is structural: fourteen MCP tools operating on captured data, a reader container with a read-only owner store, and a separate bounded importer. The macro compiler returns data; the installed addon accepts an explicit player paste/review/apply and checks client/character/combat/preimage before ordinary macro API calls. It does not evaluate imported Lua, simulate input or execute macro bodies. Inspect `src/main.rs`, `addon/MacroImport.lua`, `addon/MacroOperations.lua`, `addon/MacroUI.lua` and the deployed profile. Annotations and this document alone are not enforcement.

No game action is newly enabled by this record. Live qualification, operation-specific policy evidence and feature implementation remain outstanding in ROADMAP.md.
