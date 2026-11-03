# Read-only addon exporter (not yet live-qualified)

Core.lua captures equipment, carried bags and basic character stats on `/aoexport`. Collectors.lua adds active trait nodes, macros, bindings, action bars, profession ranks, quest-log entries, visible currencies and achievement points when their APIs exist. Currency and achievement coverage is explicitly partial. Unavailable sections carry reason codes. It does not collect mail/chat, send messages, alter inventory or consume external commands. Capture refuses combat. A failed capture retains the prior export.

The appropriate client Adapter.lua must run first. Packaging must specify the actual interface version and explicit variant. Classic has no default adapter: its Era/progression variant must be discovered before packaging. Forever's current adapter is explicitly beta and must change when a different variant is qualified.

The TOC must declare `## SavedVariables: AzerothOperatorExport`. The export is UTF-8 JSON encoded as hex in one string; this deliberately avoids implementing or executing a general Lua parser in the companion. Hex costs approximately twice the payload size, bounded at 4 MiB decoded.

After `/aoexport`, the player must reload or log out to persist the snapshot. Companion command: `azeroth-operator import <path-to-AzerothOperator.lua>`. Only files with the exact single export assignment are accepted. No game data is loaded into Git.

Lua 5.1 fixture harness verifies export/import roundtrip, values in all eleven sections, escaped strings, missing APIs and failed-capture preservation. Local development packaging is available through `tools/package_addon.py`; Python is a build/test dependency only. Production distribution through pinned BigWigs tooling remains pending. Secret-value behaviour and each actual client still require live proof.

Initial Retail development package has been installed locally with hash verification. It is not live-qualified. Start Retail, ensure Azeroth Operator is enabled, log into the character, type `/aoexport`, then `/reload`. The companion can then import `WTF/Account/<account>/SavedVariables/AzerothOperator.lua`. No game operation is triggered by the companion.
