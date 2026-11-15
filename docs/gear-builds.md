# Valid gear combinations under pinned reference constraints

`tools.gear_builds` validates complete equipment and generates combinations from
slot domains. It uses the pinned public catalogue and class/race/slot rules.
It supports the current level-60 model. It does not query a game, read private
account databases, select the active character or prove native equip eligibility.

## Explicit context, variants and slots

Context contains exactly `class_id`, `specialization`, `race`, `level` and
`professions`. The class ID is numeric. Specialisation is the exact native key,
for example `hunter`. Race and up to two distinct professions use native enum
names, for example `RaceOrc` and `Engineering`. The class/race pair must occur
in the pinned tables; level must be 60. Missing/unknown context rejects.

Equipment is exactly `{items:[17 item specifications],preserveSlots:true}`.
The positions use the native engine layout: head, neck, shoulder, back, chest,
wrist, hands, waist, legs, feet, two rings, two trinkets, main hand, off hand,
ranged. Empty slots are `{}`. Nonempty specs contain a positive `id` and optional
nonnegative `enchant` and `randomSuffix`. Unknown fields, booleans, floats and
out-of-range identifiers reject. Zero modifiers canonicalise to omission.

Inventory is a bounded list of `{item:spec,count:positive_integer}`. Every
canonical variant occurs once. Its count limits that exact variant across all
equipped slots. The generator does not credit another character, bank, mail,
auction or account implicitly. These are caller declarations; ownership stays
unverified. An omitted variant has zero declared copies.

`generate_gear_builds` takes `context`, `base_equipment`, `inventory`, and
`choices`, a list of at most 17 distinct `{slot,items}` domains. Slots are 0–16.
Each domain has 1–128 distinct canonical item specs. The last domain varies
fastest; other positions retain their base values. Optional arguments are
`offset`, `limit` (1–128, default 16), `scan_limit` (1–4096, default 4096), and
`expected_search_id`. The per-call work deadline is 45 seconds.

`validate_gear_build` takes `context`, `equipment` and `inventory` instead.
Its result distinguishes `accepted`, `rejected` and `unresolved`. Accepted means
the represented reference requirements and any necessary modifier conversion
passed. Native legality, ownership, skill ranks, weapon training, active identity
and simulation-model accuracy stay unverified.

## Checks and native conversion

The checks cover item existence and enum values, required level, class allowlist,
profession presence, faction, slot fit, armour and weapon proficiency categories,
dual-wield capability, two-handed conflicts, unique item IDs, shared limit
categories and exact physical-copy counts. Source-defined zero/no-restriction
defaults remain meaningful; missing actor information does not become approval.
The catalogue does not encode every training, profession rank or specialisation
requirement, so this is deliberately a reference-constraint result.

For modified equipment, the generator first checks known enchant IDs and item
suffix options. It then sends the complete positional spec to the existing
native probe and compares every ID/enchant/suffix with the response. The probe
must match SHA-256
`209297881300e029cc726b1734c56c0d84efdfdf205e56e27682eeea6378300f`
before and after the run. Output is parsed with duplicate-key checks and a
64-KiB read bound. Conversion failure, timeout, changed executable or lost
modifier is unresolved. No failed probe enters the accepted pool.

## Paging and integration

The search identity binds canonical actor/base/inventory/domains and all rule,
catalogue and expected probe pins. Reordering profession names is semantically
equivalent. Domain order and choice order remain significant. The response
includes half-open examined range, counts, reason aggregates, accepted references,
an `axis` for the joint manifest and `next_offset`. An empty accepted page has
`axis:null`. Result count, examined count and deadline all bound work.

`declared_matrix_complete` is true only for a complete zero-origin pass with no
unresolved coordinate. It does not cover all available gear. A terminal offset
cannot establish that prior pages ran. Native/global-best flags stay false.
Generated references use `{category:"gear",value:equipment}` and retain exact
modifier identity. They compose with the talent generator's references.

```powershell
./.venv/Scripts/python.exe -B -m tools.gear_builds generate --input "ABS_JSON" --catalogue "ABS_PINNED_CATALOGUE" --probe "ABS_PINNED_PROBE"
./.venv/Scripts/python.exe -B -m tools.gear_builds validate --input "ABS_JSON" --catalogue "ABS_PINNED_CATALOGUE" --probe "ABS_PINNED_PROBE"
./.venv/Scripts/python.exe -B -m tools.gear_builds serve --catalogue "ABS_PINNED_CATALOGUE" --probe "ABS_PINNED_PROBE"
```

The CLI reads one ordinary, single-link absolute JSON file within 512 KiB. MCP
startup fixes the two source paths. Calls expose only `validate_gear_build` and
`generate_gear_builds`, with no file/executable override. The escaped response
stays below 256 KiB. The feature fetches no source and stores no result history.
Temporary native output stays local and closes before cleanup. No MCP client
configuration changed in this slice.

## Offline evidence, 15 November 2026

The builder authenticates the same 16,703,450-byte pinned source archive used by
the talent generator. The 4,595-byte gear artifact hashes to
`d91d4abd941cd31a584fd1d5aa98b0e6ab7288727fba7d08ffebcc12c749609a`.
Its source paths/hashes identify the class capability table, native item-fit
rules, faction map and eligibility checks. Existing WoWSims MIT attribution
applies. The catalogue retains its existing exact byte pin.

```powershell
./.venv/Scripts/python.exe -B -m tools.build_forever_gear_rules --source-zip 'C:/Temp/codex/azeroth-rules/source.zip' --check 'T:/Projects/Azeroth Operator/clients/forever/gear-rules.json'
$env:AZEROTH_EQUIPMENT_PROBE='C:/Temp/codex/forever-positional-v4/artifacts/equipment-probe.exe'
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_TALENT_SOURCE_ZIP='C:/Temp/codex/azeroth-rules/source.zip'
./.venv/Scripts/python.exe -B -m unittest tests.test_gear_builds tests.test_gear_reference_oracle tests.test_generated_builds_native -v
```

The builder reproduced the artifact. All **nine tests passed in 5.965 seconds**.
The independent Node test executes the exact upstream TypeScript capability
table with pinned descriptor enums and matches every class table and dual-wield
entry. Other tests cover Cartesian domains, copies, unknown context, unique
groups, hands, modifiers, paging/resume, actual CLI/stdio MCP and probe tampering.
The public Hunter integration retains its enchants and suffix, creates two gear
and two valid talent alternatives, and executes all four joint requests through
the actual native simulator. Every result has positive DPS; no global or native
eligibility claim follows.

One independent review round used caller/failure and data/guard lenses, with
responsive Sol/high agents. Neither reported a finding. Data review compared the
actual pinned upstream eligibility function with 60,759 class/item/slot decisions
across all 6,043 catalogue items and nine classes: no mismatch. This is reference
agreement, not live-game qualification. Callers checked a 128-result SDK page
(52,749 bytes), malformed/override inputs (169-byte errors), paging and deadlines.

Jev's release advisory resolved to `jev-1.14.0`, request
`72abc7ef142d7b1a5124d6a7b56e1c2a7d7ccbba2b5652d7b924658224973b54`,
685 input / 45 output tokens, 0.311 seconds, choice `ship` (probability 0.98).
The advisory is not proof. No unresolved review finding needs a follow-up ticket.

The existing account inventory, captured-character guards and game code did not
change. The whole-case evaluator keeps its 64-coordinate limit. The separate
[native window API](joint-build-windows.md) now covers larger declared matrices.
