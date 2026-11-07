# Forever interaction map

Updated 7 November 2026. This map records source evidence and engineering scope. A callable API or a passing fixture does not qualify a live build or establish authorisation.

| Area | MCP role and current state | Evidence needed next |
| --- | --- | --- |
| Character selection | Exact name and realm filter on saved snapshots. Explicit target takes precedence. | Qualify a supported, fresh active-character signal. Until then a saved capture remains historical. |
| Owned gear | Observe equipment, bags and visible character/account bank in one export. Local MCP joins observed item locations to pinned base-item records. | Qualify real storage visibility, exact modifiers, equip rules and coverage gaps. Account bank is shared within the owner's account. |
| Equipment sets and item management | Compare and recommend manually applied changes. | Verify Forever slot legality, unique limits, enchants, suffixes, set bonuses and hand combinations. Apply codes, automated moves and inventory transactions stay blocked. |
| Talents, spellbook and stats | Export available character observations, then evaluate builds offline. | Qualify Forever talent ranks, learned spells, weapon skills, buffs used in sims and supported class/spec rotations. |
| Simulation and best-in-slot | Local catalogue and encounter presets available. | Run a pinned simulator with a reproducible baseline. Evaluate complete legal owned sets. Track phase and acquisition evidence independently for obtainable upgrades. |
| Quest and world navigation | Saved quest observations and local simulator reference zones/NPCs. | Qualify a wider, licensed and build-identified world dataset. Provide offline quest and travel plans. Leave character movement and quest actions to the player. |
| Professions, currencies and achievements | Observe available snapshot sections and plan goals offline. | Qualify recipe, profession, reputation and currency coverage. Separate shared and character-specific progress. |
| Economy and trading | Evaluate public price, recipe or loot data if its terms and provenance permit local use. | Select a permitted feed. Purchases, bids, sales, mail and trades remain player actions. |
| Macros, bindings and action bars | Read saved observations where the client supports them. | Qualify visibility and report configuration gaps. No MCP-generated Apply codes or macro execution. |
| Dungeons, raids, combat and PvP | Offline encounter, roster or preparation analysis from appropriate owner-provided data. | No live combat coaching from restricted values, input automation, targeting, queueing or play. |
| Social, guild and account administration | Public reference or owner-approved offline records only. | Chat, invitations, account changes and distribution of private data stay outside MCP scope. |

New MCP routes are denied until reviewed for data source, owner scope, outputs and side effects. The tool cannot drive game input, bypass protected values, read process memory, intercept traffic or invoke a game API. See [enforced policy](../../docs/operation-policy.md) and Blizzard's [addon policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534).

Bank discovery uses the [Forever bank API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/BankDocumentation.lua) and [container API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua) from Blizzard's generated UI documentation as mirrored by Gethe. Runtime qualification remains pending.
