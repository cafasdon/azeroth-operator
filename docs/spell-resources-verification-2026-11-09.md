# Passive spell resources — 9 November 2026

Baseline `1fa4fa7`. Source/offline capture and analysis integration. No installation,
deployment or live/native game test.

## Source reference and design

Checked the [Forever spell API definitions](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellDocumentation.lua)
and [shared structures](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellSharedDocumentation.lua)
on 9 November. Charges, cast count and cooldown getters can return restricted
values. The structures identify charge counts and public enabled/active flags;
the GCD flag is event-sensitive. This is source reference, not native qualification
or permission to recover restricted data. Capture ignores timers, duration objects,
mod rates and GCD entirely and omits unreadable/unstable primitive groups.

The scoped Lua/test writer used `gpt-6-astra` high effort; parent implemented
strict readers, summary, exact-ID action projection and actual-MCP tests. Actual
agent token usage was not exposed.

## Author verification

`cargo fmt` passed. Direct Lua 5.1/Core/export/Rust import checks:

```text
cargo test --locked --offline --test spell_resources --test spell_usability --test spell_learned_level
4 new resource tests passed, 0.07s
1 usability test passed, 0.02s
1 learned-level test passed, 0.02s
cargo build --locked --offline
Passed, 10.61s
C:/Python314/python.exe -m unittest test_spell_resources test_spell_usability test_spell_learned_level test_spellbook_search -v
Ran 4 tests in 1.986s — OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. Builds completed before
stdio checks. Lua fixtures cover zero/positive/u32-max, absent/secret/throwing/
partial/malformed/changing values, independent groups, protected getter/field
failures, exact slot identity changes, ignored timer/GCD traps, legacy metadata,
combat/other-family no-query behaviour and no gameplay calls.

Actual MCP checks 28 malformed resource values through both readers; full/legacy/
empty/missing/1,024-row books; exact reported-ID projection, numeric duplicate
slot order, base-only exclusion, independent unknowns, bounded row counts, absent
private spell markers and unchanged capture bytes. An initial test request used
`action_bars`; the existing guard correctly required `action_bar_observation`.
The fixture/request was corrected to that actual entry point; no guard was relaxed.

## Package

Built `verify_addon` as an offline example (1.58s), then packaged Forever/live/
interface 16001 version `0.44.0-dev` in `dist/forever-spell-resources-0.44.0-dev`.
SHA-256: `5981afe04ed956d278abe6af57d1ad7bbc010b413b874214d06926b4d8cf8074`.
Four-file capture-only manifest/rendered-source checks and packaged Lua 5.1
round trip passed. Forever capture fixture/changed-interface refusal true,
live qualification false. The package remains uninstalled.

## Advisory and review

Plan: `jev-1.14.0`, request
`19d0350369fddb9a2a83897524fa5e9159bd81badbab26b42269dea3886888f6`,
683 input / 47 output tokens, 0.289s. Revise 0.56, proceed 0.38,
insufficient evidence 0.06, confidence 0.34. Author tightened the action join from
base/reported references to exact reported IDs only. The split advisory remains
recorded and is not represented as approval; no favourable plan reroll occurred.

One independent review round, Callers and Failure/security, both `gpt-6-astra`
high effort, no actionable finding. Actual agent usage was not exposed. Both
ran the prebuilt resource/usability/learned-level binaries: 4 + 1 + 1 tests passed.
Failure/security ran the actual-MCP resource test (0.446s), then independently
rejected seven malformed count/Boolean/extra-duration values through Python and
actual Rust MCP, without private error disclosure. Callers ran the same resource
test plus two existing usability/learned-level MCP tests; all passed.

Callers' independent stdio probes checked action-slot and exact spell-ID filters,
unknown IDs, base-only exclusion, numeric slot 4096 and six invalid slot/count/
family/type cases. It traced the real action review caller receiving filtered
action slots and validated spellbook evidence. Both reviewers checked package
hashes and collector text against current source. Their initial raw-byte equality
assertion failed solely on CRLF source versus LF package; the existing packager's
normalised text rendering matched. No source/package drift or fix round.

Reviewers used frozen parent-built executables and synthetic snapshots, without
building, editing, network/native/game operations, deployment or SQLite. The
parent's offline build establishes the source/executable correspondence; these
reviews do not independently establish native API behaviour.

Final source/offline advisory: `jev-1.14.0`, request
`e51a6503b3c2378344d86495debb162967ce829453ba80cdf28c73eb6705b4b2`,
1,090 input / 52 output tokens, 0.294s. Land scoped 0.99, fix required 0.00,
insufficient evidence 0.01, confidence 0.98. Uncalibrated advisory based on new
implementation/test/review evidence; it does not replace the retained plan
revision result. Author disposition: land bounded source/offline functionality.
Native qualification and the larger active-character/owned-gear/world/class/
encounter/profession coverage goal remain open.
