# Operation scope and evidence

## Optional public knowledge companion

The local public knowledge stdio companion exposes only
`search_forever_knowledge`. It reads a configured, pinned public catalogue,
derived SQLite index and checksum-pinned local embedding model. Request
arguments select a query, known record section and bounded result count.
Its output supplies candidate source identities. Snapshot ingestion,
external inference, gameplay and document writes have no routes.

Reviewed 2026-11-03. This records project decisions and evidence limits, not Blizzard approval. Technical API availability alone does not establish permission. Retail export/readback is partially live-qualified; macro creation is live-qualified; other player-applied mutations have separate qualification gaps. ADR006 supersedes the original blanket read-only implementation target.

## MCP enforcement, 7 November 2026

The MCP registers an explicit offline allowlist. Snapshot listing, inspection, comparisons, character summaries, data-only equipment review, the original captured talent string and a configured Forever catalogue lookup can be exposed. It denies all seven legacy preparation routes that generated addon Apply codes, even if a client calls one directly. There is no runtime override. The separate hosted gateway profile also lists only the approved read routes.

Offline tools may use saved observations and a verified local catalogue. They must never drive input, broadcast commands across accounts, attach to a game process, intercept traffic, bypass protected or secret values, run arbitrary game code, or create a live game connection. A new route needs source review for its inputs, outputs and side effects before admission. Player involvement, technical feasibility and API availability do not establish Blizzard authorisation. Check applicable regional terms and source-specific data rights before enabling new acquisition or game-facing features.

Each stdio process binds one explicit, existing, absolute snapshot directory at startup. Configure it under the owner's OS account with private filesystem permissions. Request arguments cannot select a store. The hosted gateway authenticates with the dedicated owner's bearer credential and mounts only that owner's store read-only. Treat that credential as access to the owner's records. A separate installation or principal needs its own credential and store. The reference catalogue is separate and its sync never sends snapshot data.

This is a conservative project control. It does not establish Blizzard approval or cover pre-existing installations of the standalone addon or older MCP binaries. The historical matrix below records implementation and qualification evidence, not which write-related routes the current MCP exposes.

## Published-source recheck, 9 November 2026

The MCP capability report now explicitly sets
`blizzard_authorization_verified`, `complete_applicable_terms_review_verified`
and `absolute_compliance_guaranteed` to false. `reviewed_on: 2026-11-09` means
published-source checks and project controls only, as stated by `review_scope`.
It is not a complete applicable-terms review or a fresh legal authorisation.
`source_evidence` records each checked URL, date and retrieval outcome. Existing
legacy `sources` and enforcement fields remain available for compatibility.
No route, gameplay permission, installed addon or hosted deployment is changed.

Only the official opening posts were read as policy evidence, not community
replies. The [5 August 2025 third-party-software notice](https://us.forums.blizzard.com/en/wow/t/prohibitions-on-third-party-software/2142972/1)
reiterates restrictions on modifying the client and bypassing security. The
[UI addon policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534/1)
and [May 2021 input-broadcasting update](https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613/1)
were read directly again. These sources do not specifically approve this MCP.
The direct [EU/UK EULA](https://www.blizzard.com/en-gb/legal/2c72a35c-bf1b-4ae6-99ec-80624e1b429c/blizzard-end-user-license-agreement)
and [US EULA](https://www.blizzard.com/en-us/legal/bfbbb648-bcc6-4b78-a5c7-e2fe20c135df/blizzard-end-user-license-agreement)
requests returned HTTP 403. Indexed search text was not substituted for full
current applicable regional/account terms. Controls remain conservative project
restrictions; technical/API success and player participation do not establish
permission. [Verification](policy-evidence-verification-2026-11-09.md).

## Published sources

- [May 2021 input broadcasting update](https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613), official announcement extending the restriction to software and hardware that mirror commands or automate or streamline multiboxing.
- [Blizzard EU/UK EULA](https://www.blizzard.com/en-gb/legal/2c72a35c-bf1b-4ae6-99ec-80624e1b429c/blizzard-end-user-license-agreement), official indexed clauses reviewed on 7 November 2026. The direct request was refused. The reviewed clauses cover unauthorised automation, data mining and connections.

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
| Copy captured actions into empty slots | Development implementation deployed | Disabled | Disabled | Spell/item/macro identity and source preservation; Strict transport and shared review/receipt integration implemented; requires 0.18.0-dev; public synthetic checks passed; native unqualified |
| Move carried items | Development implementation deployed | Disabled | Disabled | Exact-instance move to explicit empty slot; first live attempt outcome_unknown, no replay; timing fixes fixture-tested |
| Split carried stacks | Development implementation deployed | Disabled | Disabled | Explicit amount, empty destination, source conservation and distinct destination identity; live unqualified |
| Swap distinct carried item IDs | Development implementation deployed | Disabled | Disabled | Exact two-item identities; no retry/cursor clearing; strict transport and review/receipts implemented; requires 0.20.0-dev with both-direction bag metadata preflight; public synthetic checks passed; native unqualified |
| Sort/merge/same-ID swaps, bank/mail transfers, sell/delete/disenchant | Disabled | Disabled | Disabled | Not yet implemented or exposed |
| Auction bids/listings, purchases, crafting, trade or chat sends | Disabled | Disabled | Disabled | No transaction or messaging tools |
| Graphics/CVar configuration proposals | Not implemented | Not implemented | Not implemented | Requires exact client settings, hardware evidence and bounded file-change workflow |
| Combat, movement, targeting, quests, queues, farming, anti-AFK | Excluded | Excluded | Excluded | User's rules-only scope; no autonomous gameplay or input automation |
| Memory reading, injection, packet interception, unlockers, protected-action bypass | Excluded | Excluded | Excluded | User prohibition and unauthorised-software concerns; never a fallback |

## Requirements for any future enabled action

Record the exact operation, client family/variant/build/interface, official policy/API evidence, player interaction required, combat/protected/secret-value restrictions, data prerequisites and observed result. Unknown evidence must remain explicit. A working fixture or successful API call is not enough to enable execution. Preserve original scope: supporting a review draft does not fulfil an equip/build/item-management request.

No game action is newly enabled by this record. Live qualification, operation-specific policy evidence and feature implementation remain outstanding in ROADMAP.md.

## Visible dialogue observation (2026-11-09)

Source-only Forever gossip_options capture reads an already visible ordinary
addon panel during the existing manual out-of-combat export. It calls neither
selection nor refresh/close APIs, reads no entered code/confirmation text, and
adds no route to the offline MCP allowlist. Labels remain untrusted owner-local
data. This captures a partial observation and supplies no NPC identity, action
eligibility or Blizzard approval. Compatible readers must accompany any later
addon rollout; no installation or native qualification occurred in this unit.

## Passive owned-auction cache (2026-11-10)

Forever source can observe the loaded player-owned auction cache during a manual
export with the auction panel already open. It reads two cache getters only;
no query, refresh, bid, post, cancellation, automatic polling or UI loading is
introduced. Exported rows contain numeric auction/item IDs, quantities and raw
status only. Third-party names, links, bids and prices are excluded. The section
is owner-local, excluded from public indexing and gives no inventory/wallet
credit. Repeated agreement does not establish freshness, complete ownership or
current transaction eligibility. This is conservative source support, not
Blizzard approval or native qualification. Auction transaction routes remain
disabled. See ADR-077-passive-owned-auctions.md.

## Passive battlefield slot status (2026-11-10)

Source may observe the reported battlefield slot capacity and raw status tokens
during a manual out-of-combat export. No queue query, join, acceptance, leave,
refresh, polling, UI loading or scoreboard request is added. Only status tokens
are retained; all other API returns remain excluded. Historical confirm/active
readings authorise no action. Owner-local storage and strict readers apply;
source fixtures do not establish native availability or Blizzard approval.
All queue-management and automation routes remain excluded. See ADR-078.
