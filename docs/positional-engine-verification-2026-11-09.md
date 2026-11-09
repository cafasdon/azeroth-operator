# Isolated positional simulator extension — 9 November 2026

Base source: `76dac88`. ADR 028 addresses the underlying automatic-placement
gap in an isolated offline engine, without changing MCP/adapter guards or
installing/replacing an engine used by the existing workflow.

## Source and build provenance

Inspected the cached pinned source archive and primary upstream
[equipment conversion](https://raw.githubusercontent.com/wowsims/forever/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/database.go),
[protobuf contract](https://raw.githubusercontent.com/wowsims/forever/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/common.proto)
and [CLI parser](https://raw.githubusercontent.com/wowsims/forever/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/cmd/wowsimcli/cmd/sim_request.go).
Automatic placement fills the first free paired slot; the old CLI discards
unknown fields. These are independent facts: the new flag must be accompanied
by an explicit verified engine-capability gate in the future integration.

Two final fresh-directory builds used the same locally cached archives/tools:

| Input | Version/identity | SHA-256 |
| --- | --- | --- |
| WoWSims source ZIP | `9dc32ac8ca0d8d1b76337861d38b93c0f884af11` | `e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712` |
| Go Windows amd64 ZIP | `go1.27.3` | `f26081f438319757f8a1b46d7d0b2c5ec7583d4038e485e0aa725cfde7885fff` |
| protoc | `36.2` | `5a1b5350308309c9729ce4484a798981f281c10909144c4bdefd6a37688e4b1f` |
| Go protobuf plugin | `v1.36.10` | `974ecc9f65c335445167d6e4033f1e89f24b9d455511f5dae07b916ed6a47bb0` |

The Go archive hash/size matched the official [Go download manifest](https://go.dev/dl/?mode=json&include=all).
Research fetched public metadata/text only; builds reused local archives,
protobuf executables and module/build caches. SDK restoration is confined to
the fresh build directory, not a system installation. No upstream makefile,
CI workflow or generation script was run; protoc and scoped Go builds are
called directly with offline settings.

Final artifact identity:

- Engine: `833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
- Equipment probe: `209297881300e029cc726b1734c56c0d84efdfdf205e56e27682eeea6378300f`.
- Builder: `2f8578c08844958551670c18b185ed42596ec1fb7e3e3cf2ce9dc3ea3b915dde`.
- Constructor fragment: `c9765a642f654fa0999cfdea5a3dfc376f7bfef69fae0518bc6da98326d0c1e5`.

The complete manifests from fresh `forever-positional-v4` and `v5` work
directories were exactly equal, including binary/probe hashes. Existing source
archives, incumbent executable and installed MCP were not replaced. Build
manifests intentionally retain `qualification_performed: false`; evidence below
is separate from compiler success.

## Author verification

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_QUALIFY_ENGINE='C:/Temp/codex/forever-sim-9dc32ac/wowsimcli.exe'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
C:/Python314/python.exe -m unittest test_positional_engine_build test_real_positional_engine -v
```

Seven tests passed in **13.726s**. Without explicit local artifact/template
environment values, real-engine tests skip and never download or build.

- Builder unit tests reject traversal, absolute/drive/backslash and case-alias
  ZIP entries and expansion overflow; patching requires exact expected anchors.
  Windows ZIP filename normalisation initially hid a malformed test backslash;
  the guard now checks original member names and the fixture preserves raw names.
- Poisoned Go environment values cannot enable direct proxy/VCS fetching,
  overlays, workspace substitution, external cache helpers or another target.
- Mocked timeout/interruption tests verify tree-kill calls. A separate actual
  Windows test starts only owned Python parent/grandchild sleepers, times out
  the parent and verifies both process handles are terminal/missing afterward.
- Actual numeric probes preserve first/second rings, first/second trinkets and
  one-hand off-hand-only layouts. Known enchant 1900 on item 2042 and suffix
  13012 on ring 9461 retain exact IDs. Bad lengths, wrong slot fit, empty-slot
  modifiers, unknown IDs, incompatible enchants and two-hand/off-hand conflicts
  reject with generic errors. These are conversion probes, not qualification of
  all such combat configurations or character eligibility.
- Actual canonical level-60 public Hunter profile, 1,000 iterations and fixed
  seed 20261109: incumbent, new legacy mode and new positional mode return
  identical numeric metrics. Mean DPS is `944.1209054284079`, standard deviation
  `58.83234825602769`; distinct engine hashes remain recorded, not compared as
  if the executable identity were equal.
- A sparse second-ring-only layout with known main/ranged items executes in the
  new exact mode, mean DPS `650.883932442968`. This is a model execution check,
  not an upgrade recommendation. The incumbent accepts but ignores the flag
  and rejects the new capability command, proving flag-only detection unsafe.

The source fragments contain no character/owner data. No game client, native
qualification, addon installation or deployment occurred. Full model accuracy,
native eligibility, complete ownership, all-class/encounter coverage and BiS
remain unverified. MCP/adapter integration is the next dependent task; current
owned-gear workflow still honestly reports its paired-slot gap.

## Independent review and fixes

Two independent `gpt-6-astra` agents at high effort covered Callers and
Failure/security. Round one identified inherited Go configuration defeating
offline/native-target guarantees and silent unknown-enchant loss under the
preserve-or-reject contract. Both are inherited mechanisms but are scoped
faults for this new builder/mode. Author reproduced enchant 2147483647 on ring
4549 returning enchant zero before fixing; the new regression failed before
the fix. Go's cached proxy-routing source independently establishes the direct
fetch path; no network attack was executed.

The builder now seals Go configuration/workspace/proxy/private/VCS/telemetry/
target/flags/cache-helper settings. Exact mode checks returned ID/modifiers and
existing enchant-fit rules; legacy mode remains unchanged. Fresh builds above
contain those fixes. Round two found no remaining blocking issue.

Callers ran five builder tests in 2.024s and the authorised capability/numeric
probe test in 2.197s. It replayed unknown enchant on item 2042: exact mode rejects,
legacy returns zero; valid 1900 stays accepted. Independent poisoned settings
returned explicit offline/native values. Failure/security ran six builder/
numeric tests in 4.036s, including actual owned-child timeout termination. It
replayed ring 4549 with enchants 2147483647, -1 and incompatible 1900: all reject
in exact mode, while legacy retains its old permissive behaviour. Neither
reviewer built software or ran raid simulations, HTTP, game actions, Git
mutations or deployment.

A nonblocking helper limitation was recorded: a synthetic Windows `go/NUL` ZIP
member writes to a device rather than creating a file. Arbitrary ZIPs cannot
reach `build()` extraction because exact archive hashes are required. No claim
of a general-purpose untrusted archive extractor is made.

The completed docs review identified a provenance wording overclaim: the
fragment README said all input hashes were recorded, although dependency/build
cache contents are trusted and unhashed. It now lists the exact hashed categories
and explicitly excludes those cache contents. No runtime change was needed.
Callers independently checked v4/v5 manifest byte equality, actual output hashes
and documented builder/constructor hashes; Failure/security also matched every
documented output/builder/constructor hash to the manifest. Author-only raid
results remain clearly attributed. Both reviewers directly confirmed the corrected
README/evidence wording afterward; no documentation finding remained.

## Advisory receipts

All results below are uncalibrated advisory values, not proof or permission:

- Explicit contract plan: `jev-1.14.0`, request
  `e0a8edded58276d7dbe2b3316e5e9291881b48168a9254a2e38131a4fa900296`,
  861 input/53 output tokens, 0.268s; explicit contract 1.0, confidence 1.0.
- Unknown-field discovery: `jev-1.14.0`, request
  `3fd1bce4e5dc68fd3d74560bf87954f4e06ecdd6cac43055a73ccacea52e1332`,
  629/57 tokens, 0.278s; require explicit capability 1.0, confidence 1.0.
- Finding triage: `jev-1.14.0`, request
  `6c27347a9174732fa18fbaef41395787f083ebe1e82c3c15dce8dd1dfc47f36a`,
  854/88 tokens, 0.285s; fix each fault 1.0, confidence 1.0.
- Provenance wording triage: `jev-1.14.0`, request
  `077cfe39abcc016263acd1a0f602d11d7e7a9fd1c1d74df22b051711c93170ac`,
  489/52 tokens, 0.300s; correct wording 1.0, confidence 1.0.
- Final scoped landing: `jev-1.14.0`, request
  `3e9b1304c1b16256c95ab1b32b5271c0321ecb2ead483a9040b001876f3d5516`,
  1,093/49 tokens, 0.275s; land-scoped 0.97, fix-first 0.02,
  insufficient-evidence 0.01, confidence 0.96.
