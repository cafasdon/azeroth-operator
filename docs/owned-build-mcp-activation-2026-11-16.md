# Owned optimiser MCP activation, 16 November 2026

`azeroth-owned-builds` adds three owner-local stdio tools around the reviewed
[saved source](ADR-112-owned-build-source.md) and
[owned optimiser](ADR-113-owned-build-optimisation.md):

| Tool | Behaviour |
| --- | --- |
| `list_owned_build_characters` | Page account-scoped saved characters and current-capture storage counts. |
| `prepare_owned_build_search` | Bind the saved export character by default, or an explicit name and realm, then prepare a bounded owned matrix. |
| `evaluate_owned_build_window` | Evaluate up to 64 coordinates from a remembered search after another source check. |

The current saved addon export supplies the default identity. This means the
character active **at that export**, not verified current play. An explicit
name and realm override it. The account key, exact client, snapshot directory,
saved export and reader path stay in private local configuration. An unindexed
new export rejects until the existing owner ingestion workflow updates the index
and hash-bound store. No newest-alt fallback or silent public model substitution
exists. Capture age is recalculated on each response.

The [sanitised MCP table](../clients/forever/mcp-owned-build-tools.example.toml)
contains only launch paths and the exact three enabled tools. The existing
public-fixture and generic gear/talent tools remain available separately.
The owner entry fixes the project virtualenv, working directory, `PYTHONPATH`,
`GOMAXPROCS=2`, 30-second startup and 300-second call timeout. Native files and
the private bounded numeric cache use the previous durable runtime bundle.

The server requires a regular, single-link, owner-private source JSON file.
Its exact keys are `reader`, `snapshots`, `active_export`, `inventory_database`,
`account_key`, and `client`. Paths are absolute. The client has exact family,
variant, build and interface fields. The configuration can contain private
identity paths; never commit its actual contents. Only its ACL changes during
installation; existing captures, account inventory and game files remain untouched.

The installer adds only the owned entry, with the existing exclusive Windows
compare/write, private backup, saved-byte readback and native parser checks:

```powershell
./.venv/Scripts/python.exe -B -m tools.install_build_mcp --owned-only --config 'C:/Users/cafas/.codex/config.toml'
# To apply, also supply --apply, --stage-dir ABS_PRIVATE_STAGE and
# --codex-launcher node ABS_CODEX_JS. Provision the private source/runtime first.
```

The helper rejects conflicting existing entries and any unknown target name.
It preserves unrelated configuration and does not enable other disabled tools.
Its existing in-place update is not crash atomic; the private original backup
supports recovery. Full configurations and backups never enter Git.

The tools reject paths, inventory, context and account overrides in arguments.
Replies have a 256 KiB limit and generic errors. Complete prepared models remain
inside the existing eight-case, 4 MiB process-memory limit. A cancelled call
keeps its bounded worker busy until completion; another call cannot race it.
Restarting the server loses prepared searches; prepare them again.

The level-60 DPS model restriction remains. Unsupported saved levels return
bound plan-only reports and explicit reasons. Shared banks, mail, old bank
observations and other characters do not become available gear copies.
Current play, current ownership, complete inventory, native legality, model
accuracy and global-best claims remain unverified. No live test is part of this
activation. An existing chat can need an MCP refresh to see a newly added server.

## Offline acceptance

```powershell
./.venv/Scripts/python.exe -B -m unittest tests.test_owned_build_mcp -q
# Set AZEROTH_MCP_CONFIG to test the actual installed entry too.
# Set AZEROTH_NATIVE_MCP_SCOPE=owned, AZEROTH_MCP_CONFIG and a private
# AZEROTH_NATIVE_CODEX_HOME for isolated native-client tool discovery.
./.venv/Scripts/python.exe -B -m unittest tests.test_native_build_config -q
```

Synthetic SDK calls passed all three routes, native positive scores, repeat cache
hits, generic private errors and the unsafe-config ACL guard. The 22 existing
installer guard tests also passed. Independent reviewers requested as Astra/high
covered callers and security/data with real stdio calls, projection, cancellation,
bad inputs and additive registration.

The [installed acceptance record](benchmarks/owned-build-mcp-activation-2026-11-16.json)
confirms the actual saved commands: default and explicit selectors bind the same
private capture, its unsupported level stays plan-only and no game action runs.
The installed SDK check passed in 3.416 seconds. Fresh native Codex discovery of
all three tools passed in 3.922 seconds with the exact installed entry in an
isolated private home, without account credentials or model requests.

Actual byte readback confirms all 21 previous MCP servers and unrelated settings
remain unchanged. Only `azeroth-owned-builds` was added. The source JSON is now
owner-private, its bytes stayed equal, and the private backup matches the previous
client configuration. Native parsing, LF bytes and idempotent planning passed.
This open chat's catalogue still lacks the new namespace; refresh or restart the
MCP client to expose it. No current-chat hot reload or app restart is claimed.
