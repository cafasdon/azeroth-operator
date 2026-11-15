# Valid talent combinations under pinned reference rules

`tools.talent_builds` validates and generates talent allocations for the nine
Forever classes. It supports levels 1–60 for talent point constraints. The native
simulator still supports only level 60; lower-level talent checks do not enable
lower-level combat simulations or establish native game eligibility.

## Contract

`validate_talent_build(class_name, level, talents_string)` checks every rank,
the budget `max(0, level - 9)`, row gates and prerequisites. A talent in row N
needs at least `5*N` points in earlier rows. Every purchased prerequisite must
have its maximum rank. A bad final allocation returns `valid:false` with bounded
violation codes. Malformed shapes, unknown classes and unsupported levels reject.
Strings have at most three trees and 256 characters. Missing trailing ranks are
zero. The returned canonical form preserves three tree positions.

`generate_talent_builds` accepts exactly:

```json
{
  "class_name": "hunter",
  "level": 60,
  "base_talents": "5320001505101251-0052050301-4",
  "choices": [
    {"definition_id": 134422, "ranks": [4, 5]},
    {"definition_id": 134438, "ranks": [3, 4]}
  ]
}
```

Optional fields are `offset` (canonical decimal string), `limit` (1–128,
default 32), `scan_limit` (1–4096, default 4096) and `expected_search_id`.
There are at most 16 distinct definition domains. Ranks are unique strict
integers within that definition's maximum. The last domain varies fastest.
Unchanged nodes retain their base ranks. A syntactically valid but broken base
can be repaired by its domains; it is not automatically accepted.

The result includes explicit class/level, the search/rule hashes, half-open
examined range, accepted/rejected counts, aggregated reasons and `next_offset`.
Accepted rows contain the canonical string and SHA-256 of
`{category:"talents",value:string}`. `references` and `axis` plug directly into
the existing joint manifest. A page without accepted allocations has `axis:null`.
Generation stops at either the accepted-result or examined-coordinate budget.

Resume must supply the returned offset and search identity. A change to class,
level, semantic base, domain order/ranks or rule hash rejects the old identity.
`declared_matrix_complete` is true only when this call starts at zero and examines
the entire declared matrix. A terminal cursor does not prove prior work. No
complete class talent universe, personal snapshot or globally best build is
claimed. All native legality, model accuracy and live-action flags stay false.

## CLI and MCP

```powershell
./.venv/Scripts/python.exe -B -m tools.talent_builds generate --input "ABS_JSON"
./.venv/Scripts/python.exe -B -m tools.talent_builds validate --input "ABS_JSON"
./.venv/Scripts/python.exe -B -m tools.talent_builds serve
```

CLI input must be an ordinary, single-link absolute file within 512 KiB.
Duplicate keys and unknown fields reject. The MCP exposes the two named
functions with strict argument checks and no path/executable overrides.
Its escaped text envelope remains below 256 KiB. Errors do not echo rejected
payloads. The generator stores no profiles or result history and contacts no
external source. Explicit class/level inputs are model context; this tool does
not choose or act on a logged-in character. Existing character selectors and
captured-talent matching remain unchanged.

## Source and verification, 15 November 2026

The restored public source archive is 16,703,450 bytes and exactly matches the
existing pin `e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712`.
No source download occurs during the feature's execution or tests. The builder
authenticates that archive and reuses the identity-map builder's nine-class
protobuf/tree cross-checks. The result is 35,827 bytes, rule SHA-256
`f864f3f2fc197c9844004ee56de4b69818fdd6c123c886e9ce5daf5cf9245d34`.
Source paths/hashes accompany the artifact. Existing WoWSims MIT attribution
under `clients/forever/third-party-WoWSims-LICENSE.txt` covers these derived data
and the exact upstream test fixture. LF attributes preserve byte identities.

```powershell
./.venv/Scripts/python.exe -B -m tools.build_forever_talent_rules --source-zip 'C:/Temp/codex/azeroth-rules/source.zip' --check 'T:/Projects/Azeroth Operator/clients/forever/talent-rules.json'
$env:AZEROTH_JOINT_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
./.venv/Scripts/python.exe -B -m unittest tests.test_talent_builds -v
$env:AZEROTH_TALENT_SOURCE_ZIP='C:/Temp/codex/azeroth-rules/source.zip'
./.venv/Scripts/python.exe -B -m unittest tests.test_talent_reference_oracle -v
```

The six focused tests passed with the native engine enabled. The four-coordinate
Hunter example accepted exactly the two known valid allocations, rejected one
row violation and one 52-point allocation, and preserved resume/reference IDs.
Both accepted references ran through the actual native joint evaluator with
positive DPS and no global-best claim. Actual CLI and SDK stdio success/error
calls passed. The exact upstream TypeScript function agreed on all 720 recorded
states across nine classes. This checks reference semantics, not combat accuracy.

Independent caller and data reviewers also tested 4,500 dense/sparse states,
540 dense valid/damaged cases, nine-class paging, hash re-computation and SDK
failure limits. No medium/high finding remained. A 128-accepted-result MCP probe
fit in 43,550 bytes before the two small context fields were added. Those fields
were added only to make class/level explicit. No identity or game source changed.
Reviews used Sol/high after the Astra worker did not initialise; Astra ran no
implementation. Model resolution/token totals are not exposed by this harness.

Jev's advisory plan resolved to `jev-1.14.0`, request hash
`33f0794888a6a9a64de17717ee18d6698b4296f46631a97fa64e1e28624087b1`,
672 input / 47 output tokens, 0.298 seconds, disposition `proceed`.
The release advisory resolved to the same model, request
`e77637db64f6a30a02dfab1f40e60ffe0a043fae05d458f0efa8fc0aea818a9e`,
583 input / 48 output tokens, 0.324 seconds, choice `ship` (probability 0.68).
These are advisory decisions, not proof. Final author execution passed all
seven tests in 2.750 seconds with the native and source-oracle options enabled.

This module validates talents. [Gear generation](gear-builds.md) and
[larger native windows](joint-build-windows.md) are separate tested features.
No game test or MCP client configuration change occurred in this slice.
