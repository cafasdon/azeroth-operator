# TomTom route drafts

The local route companion prepares up to 64 numbered waypoints with explicit
native UIMapIDs and percent coordinates. It does not read game state or control
TomTom. Its one tool is `prepare_tomtom_route(title, steps, active_step, mode)`.

Each step has exactly these fields:

```json
{"ui_map_id":37,"x_percent":51.2,"y_percent":34.8,"label":"First quest stop"}
```

These are format examples, not verified quest coordinates. A database area ID
is not a UIMapID. The tool rejects extra fields such as provider area IDs rather
than infer a conversion. Native map existence, correct floor/phase and current
accessibility remain unknown.

`stepwise` is the default mode. It returns import text for `active_step` only,
starting at one. Request the next index manually when appropriate. `all_pins`
returns every pin in the supplied order, with numbered/hash-qualified titles.
This is text order only: TomTom's closest-waypoint selection can choose a
different pin. Even stepwise mode does not verify that the arrow targets the
selected pin. Existing pins and TomTom profile settings remain unchanged.

The user reviews the draft, opens `/ttpaste` and pastes the text manually. No
tool pastes, executes, moves, shares waypoints, resets existing pins or advances
quest objectives. A proper ordered arrow/Next controller remains unimplemented.
Neither arrival nor a click is treated as proof of quest completion.

Titles and labels have an 80-byte UTF-8 limit. Inputs reject controls, format
characters, line separators, hyperlink markup, invalid types and non-finite or
out-of-range coordinates. The output shows original and five-decimal rendered
percent values with a rounding flag. `pin_label` is the literal paste suffix.
When TomTom can change numeric punctuation, the manifest preserves the requested
text and gives `display_pin_label_variants` for dot/comma numeric locales, with
`parser_may_change_pin_label: true`. These are version-qualified predictions,
not a live display check. Full route hashes and step numbers prevent
identical-position steps from sharing a pin title.

The 64 KiB transport limit includes the text result envelope and escaping.
Oversized routes fail with a bounded error instead of silently truncating.
Route labels may contain private information; the companion keeps no history
and sends no request to a data provider. It is an owner-local process, not a
shared service.

## CLI

Store an explicit `title` and `steps` object in an ordinary local JSON file.
Optional keys are `active_step` and `mode`. Duplicate JSON keys fail.

```powershell
./.venv/Scripts/python.exe -B -m tools.tomtom_route prepare --input "ABSOLUTE_ROUTE_JSON"
./.venv/Scripts/python.exe -B -m tools.tomtom_route serve
```

The MCP accepts no file paths, SQL or arbitrary addon commands. Its advertised
schema rejects extra properties, including in steps. Manual strict validation
also runs in the handler, so invalid large inputs produce short errors rather
than SDK validation messages that echo the full input.

## Source evidence

The [publisher's project page](https://www.curseforge.com/wow/addons/tomtom)
documents multiple waypoints and `/ttpaste`. Its
[source page](https://www.curseforge.com/wow/addons/tomtom/source) identifies the
publisher's Git repository. The installed Forever version is v4.4.1-release and
its TOC declares interface 16001. The installed README documents `/tway #ID`
with percent coordinates, and `AddWaypoint(map,x,y,options)` with normalised
coordinates. Addons must not confuse these two scales.

Installed `modules/slash.lua` resolves explicit `#ID` through its map table and
divides percent coordinates by 100 before `AddWaypoint`. `modules/paste.lua`
removes the command prefix and sends each line to that parser. The arrow module
can select the closest remaining waypoint; it is not a sequence guarantee.
Only file hashes and our integration code are committed, not publisher assets.

## Offline acceptance — 15 November 2026

Nine focused tests passed, including actual CLI and stdio SDK calls, repeated
positions, mode/index selection, strict schema/types, precision disclosure,
Unicode/command injection and file/transport bounds. Independent caller checks
returned a 52,673-byte 64-step Unicode SDK response. Three invalid/oversized SDK
responses were 154 bytes each and did not echo the input. These initial envelope
measurements preceded the added label-prediction fields; the same transport cap
still applies. Independent security
checks passed 240 seeded drafts and rejected 74 control/format/surrogate and
noncharacter cases, with file, process and network calls blocked.

The optional installed-parser test passed with an explicit
`AZEROTH_TOMTOM_DIR`. It first matched all five source hashes and the pinned Lua
interpreter hash, then loaded the installed slash module in an isolated Lua
environment without game, file, network, process or loader APIs. Synthetic map
tables and an `AddWaypoint` collector verified percent conversion, explicit map
IDs, numbered titles, numeric-punctuation display predictions, return-to-same-point
steps and literal text resembling a command inside a label. Its environment
explicitly qualifies the dot numeric locale. No waypoint was created in the game. This does not
verify actual map identities, addon loading, persistence or native placement.

The architecture and execution boundary are in [ADR-106](ADR-106-offline-tomtom-route-drafts.md).

## Review and local registration

Two bounded review rounds covered callers/failures and security/data. The
installed-parser fixture received a focused source comparison. That comparison
found one medium label-fidelity defect: ordinary numeric punctuation changed
between paste text and the actual parser title. The manifest now preserves
requested text and explicitly predicts both numeric-locale display forms.
The recheck closed that finding; eight more punctuation cases matched the parser.
Expanded-label checks accepted 47 bounded drafts and rejected 17 oversized ones.
No medium/high or low finding remained. No live infrastructure lens applied.

The local `azeroth-routes` registration enables only `prepare_tomtom_route`.
It changes no existing MCP entry. Explicit UTF-8 byte writes, an ignored private
backup, semantic comparison, byte readback and native parser checks protected
staged/saved configuration. A saved-configuration SDK launch from an independent
directory discovered the one strict tool and passed selected-step and three-pin
synthetic draft calls. No game or hosted service was started. Existing app
connections may need a refresh to discover the new local companion.

Jev `jev-1.14.0` advisories were recorded as follows. They are advice, not native
acceptance or permission to operate the game:

- Design proceed: `3788590352b6375842785d8c58c95736c09e266ab61da510a12590ffe111e022`,
  791 input / 47 output tokens, 0.319 seconds.
- Label-fidelity medium: `1c24f47cdb1eaaadb2ae42addb480953d30fdc3a6146d04a3e39d65e24ae5624`,
  686 input / 51 output tokens, 0.285 seconds.
- Release ship: `707c59e47eb44913dd7ffd38154b8bc6605c0832ed2eab90df712994ca46e0c0`,
  847 input / 44 output tokens, 0.303 seconds.

The main session accepted the bounded implementation, the reproduced fix and
direct/independent checks as the release basis. Live tests stay deferred.
