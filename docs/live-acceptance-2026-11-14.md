# Forever installed-path acceptance — 14 November 2026

## Result

The logged-in Forever character completed the normal capture, persistence,
restricted upload and authenticated hosted MCP readback path. This is installed
runtime evidence, not just a source or fixture result.

The client reported build 70938 and interface 16001. The four-file capture-only
addon package was installed and loaded after `/reload`. `/aoexport`, followed
by `/reload`, saved a 132,226-byte export. The strict reader accepted it.
The capture contained 35 observed sections, 13 equipment slots, 17 carried item
stacks and 16 quests. The native character panel matched the captured level-22
Shaman identity. Character names, realm, raw exports and screenshots remain in
ignored local acceptance storage; they are not published here.

## Installed components

- Capture addon: `0.21.0-livecheck`, Forever live, interface 16001. Archive SHA-256:
  `cf9a83b918474402101f80c33bc0f4800e08d22beb61d8c4035f39d86f33c260`.
- Windows runtime: the previous installed binary rejected the new export with
  `Unknown field`. The current offline release build passed an isolated real
  export import, then replaced the sync configuration's binary pin. SHA-256:
  `92bc2c502692b4d16440ac6f869c20f4d191a1d35f4c22addab1c7d31b4d39d6`.
  The normal sync command reported `uploaded` for Forever and `unchanged` for
  Retail. The existing scheduled runner uses this same configuration.
- Personal pylon CT290 reader and restricted importer: source revision
  `89425a0`, image SHA-256
  `747027947fc29955120d81f87dd95366fd33f6d5c8a7b0dab588ca65d79c21c1`.
  Both upgrades passed their direct deployment checks. The fixed owner store,
  read-only reader mount, token protection and firewall scope were retained.
- The hosted reader now discovers all 12 default tools. Public TLS checks
  returned 401 for missing and wrong tokens, accepted the correct token and
  rejected all seven tested preparation routes.
- Independent MCP SDK calls used the saved Codex configuration and read the
  uploaded capture by its exact ID. `inspect_snapshot` and `character_summary`
  agreed with the imported capture and observed item counts. The connected MCP
  also returned the same character summary.
- Local world and knowledge companions each exposed two read-only tools from
  an independent working directory. World category search and NPC/item joins
  passed. Knowledge search and exact record lookup passed without downloads.

## Review and limits

Two independent read-only reviews covered the item category change and the
installation/security paths. Neither found a confirmed medium or high issue.
Direct world MCP tests and an independent full-cache category oracle passed.
No CI workflow ran. Jev did not run; the reviews used the available agent route.

The hosted profile is offline analysis of saved captures. It does not execute
game actions or prove that a saved character is still the active character.
Closed bank, merchant, mail and profession panels did not supply those sections.
Player resources were reported unavailable by the current capture API path;
native health and mana display alone does not validate that collector.
Wallet capture remains opt-in. No complete best-in-slot or level-22 Shaman
simulation result was established by this acceptance test. Native combat,
trades, purchases, social messages and inventory mutations were not tested.

The installed local companions are configured for future Codex connections.
An existing connection can need a reconnect before its tool allowlist refreshes.
