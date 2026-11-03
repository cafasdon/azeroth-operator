# Read-only addon exporter (not yet live-qualified)

Core.lua captures equipment, carried bags (when the container API is available) and basic character stats on `/aoexport`. It does not collect mail/chat, send messages, alter inventory or consume external commands. Capture refuses combat. A failed capture retains the prior export.

The appropriate client Adapter.lua must run first. Packaging must specify the actual interface version and explicit variant. Classic has no default adapter: its Era/progression variant must be discovered before packaging. Forever's current adapter is explicitly beta and must change when a different variant is qualified.

The TOC must declare `## SavedVariables: AzerothOperatorExport`. The export is UTF-8 JSON encoded as hex in one string; this deliberately avoids implementing or executing a general Lua parser in the companion. Hex costs approximately twice the payload size, bounded at 4 MiB decoded.

After `/aoexport`, the player must reload or log out to persist the snapshot. Companion command: `azeroth-operator import <path-to-AzerothOperator.lua>`. Only files with the exact single export assignment are accepted. No game data is loaded into Git.

Lua 5.1 fixture harness verifies export/import roundtrip, item/bag values, escaped strings and failed-capture preservation. Pending: packaging, remaining snapshot sections, secret-value handling validation and live-client proof. Do not interpret fixture success as installed or working in WoW.
