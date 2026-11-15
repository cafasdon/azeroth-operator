# Offline build MCP activation, 15 November 2026

The additive setup registers three owner-local stdio servers and six tools.
It preserves all pre-existing configuration values, including disabled servers,
tool restrictions and connector settings. It does not change the hosted reader,
the addon or the game client.

| Server | Enabled tools | Startup / call timeout |
| --- | --- | --- |
| `azeroth-talents` | `validate_talent_build`, `generate_talent_builds` | 30 / 30 seconds |
| `azeroth-gear` | `validate_gear_build`, `generate_gear_builds` | 30 / 60 seconds |
| `azeroth-joint-public256` | `evaluate_joint_build_window`, `merge_joint_build_windows` | 30 / 150 seconds |

Each entry fixes the project virtualenv Python, module, working directory and
`PYTHONPATH`, with `GOMAXPROCS=2` and an explicit two-tool allowlist.
The joint server fixes its manifest, engine and private numeric cache at startup.
Tool calls cannot override these paths. Its descriptions identify the public
model case and state: `Current character is unbound`.

## Public case and durable runtime

The [public hunter fixture](../clients/forever/fixtures/README.md) contains 128
gear variants and two talent allocations: 256 coordinates. It is a level-60 DPS
model case with two trials per native request. It contains no private character
capture and does not select or bind the current character. Native eligibility,
ownership, model accuracy and a global optimum remain unverified.

Native executables reside under the project, outside the temporary build tree:

```text
local-data/forever-build-runtime/833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983/
  wowsimcli-positional.exe
  equipment-probe.exe
```

| File | SHA-256 |
| --- | --- |
| `wowsimcli-positional.exe` | `833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983` |
| `equipment-probe.exe` | `209297881300e029cc726b1734c56c0d84efdfdf205e56e27682eeea6378300f` |

The engine and gear adapters enforce these pins. The gear server also uses
`local-data/forever/catalogue.json` under the existing catalogue lock.
The joint server uses `local-data/forever-build-runtime/numeric-cache` with the
existing private-cache checks and 128-entry limit. The installer registers
paths; it does not build, download or provision these prerequisites.

## Installer and configuration

[`clients/forever/mcp-build-tools.example.toml`](../clients/forever/mcp-build-tools.example.toml)
is a sanitised example with only the three new entries. It contains no account
credentials. Do not replace the full client configuration with this example.

Run the installer from the project root. The default command only plans changes:

```powershell
./.venv/Scripts/python.exe -B -m tools.install_build_mcp --config 'C:/Users/cafas/.codex/config.toml'
```

The Windows apply route needs an existing private staging directory on the same
volume as the configuration and an explicit local Codex launcher. For example,
after the staging directory passes the ownership and DACL checks:

```powershell
./.venv/Scripts/python.exe -B -m tools.install_build_mcp --config 'C:/Users/cafas/.codex/config.toml' --apply --stage-dir 'C:/Users/cafas/.codex/mcp-activation/azeroth-builds' --codex-launcher node 'C:/Users/cafas/AppData/Roaming/npm/node_modules/@openai/codex/bin/codex.js'
```

The installer rejects conflicting target entries, malformed UTF-8/TOML,
linked paths, hard-linked files and unsafe staging directories. It preserves
unrelated parsed values exactly, normalises valid CRLF text to LF and checks
the saved candidate bytes. Repeating a successful install adds no entries.
Output contains counts, added names and the private backup path, not config
contents or native-parser output.

The actual native Codex parser runs against the original configuration, the
saved candidate and the installed result through `features list`. A private
`before.toml` backup retains the original bytes. Candidate files and backups
remain inside private staging directories; they can contain credentials and
must never enter source control or a public evidence record.
The update preserves the shared configuration's existing ACL. Only staging
directories and backups were checked as owner-private; the shared config was
not qualified as owner-private.

The final update uses a Windows handle with no read, write or rename sharing.
It compares the original bytes, writes and truncates, calls `fsync`, then reads
back through that same handle. A partial write error triggers a restore through
the still-exclusive handle. A later rollback acquires exclusion again and
compares the candidate before restoration, preserving another writer's change.
Busy readers or writers can cause a safe failure during the brief exclusion.
This is not a crash-atomic rename: an abrupt process or host failure can leave
a partial file that needs recovery from the private backup. POSIX mutation is
not supported. Native parser processes run outside the exclusive section.

## Installed evidence and current-chat limit

The [curated acceptance record](benchmarks/mcp-build-activation-2026-11-15.json)
records **22 installer tests passed in 3.004 seconds**. Coverage includes
the actual CLI, private backup, parser failures, byte readback, idempotency,
hard links, real second-process write/rename exclusion, drift before acquisition,
same-handle partial-write repair and preservation of later edits on rollback.

The actual installed configuration preserves all 18 existing server entries
and all other parsed settings. The exact previous bytes remain its prefix;
the three new tables are additive. The private backup matches the original.
The native parser accepted the original, candidate and installed configuration,
and an idempotent plan reported no additions.

SDK calls through all six installed tool routes passed in 4.244 seconds,
including rejected path overrides, joint evaluation and receipt merging.
A second installed SDK run passed in 4.360 seconds: offset 127, limit 2 gave
two cold cache misses, then two warm hits with identical scores.

Fresh native Codex discovery passed in 3.535 seconds with all six tools.
This check loaded the exact three installed entries into an isolated private
home, with apps and analytics disabled and no account credentials in that home.
It used documented JSON-RPC discovery requests without a model request.

The focused checks are:

```powershell
./.venv/Scripts/python.exe -B -m unittest tests.test_install_build_mcp -v
# Set AZEROTH_MCP_CONFIG to the explicit saved config before either opt-in check.
./.venv/Scripts/python.exe -B -m unittest tests.test_configured_build_mcp -v
# Also set AZEROTH_NATIVE_CODEX_HOME to an isolated private directory.
./.venv/Scripts/python.exe -B -m unittest tests.test_native_build_config -v
```

The current desktop chat's tool catalogue still omits the new namespaces.
Refresh or restart the MCP client to load them into this open chat. Fresh
native discovery is proved; current-chat hot reload and an app restart are not.
No game execution or live-character qualification is claimed.
