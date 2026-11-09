# Passive loot window — 9 November 2026

Baseline `8461b09`. New source/offline Forever observation system. No game action,
installation, deployment or live/native test.

## Source reference and contract

The [Forever native UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/LootFrame.lua),
checked 9 November, consumes slot count and the third slot-info return as quantity
at lines 208–209 and uses slot hyperlinks at line 63. This is an API usage reference,
not native availability evidence or policy approval. Initial guessed Shared,
Classic and Vanilla paths returned 404; the actual Forever branch Mainline path
resolved. Other search results were not used to define the contract.

Capture emits only stable public item IDs/quantities from an already shown panel,
up to 64 contiguous slots. Non-item/restricted/unreadable/unstable rows are explicit
unknowns. Counts/context must remain stable around both full reads. Raw links,
names, source entities/GUIDs and other tuple returns are discarded. Currency and
money interpretation is deliberately unresolved. Loot is not owned inventory.

The scoped Lua/Core/test writer used `gpt-6-astra` at high effort; parent owns
strict readers, summary and Python MCP tests. Actual agent usage was not exposed.

## Author checks

`cargo fmt` and the ordinary `cargo build --locked --offline` passed (initial
build 12.96s; final ordinary rebuild 2.20s).

```text
cargo test --locked --offline --test loot_window
3 passed, 0.05s
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
1 passed, 0.23s; 1 unrelated filtered out
C:/Python314/python.exe -m unittest test_loot_window test_player_auras test_gossip_quests -v
Ran 3 tests in 1.377s — OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. Lua tests execute actual
Collectors/Core through the manual export and Rust import in Lua 5.1. They cover
shown-empty/hidden/closing windows, missing APIs, secret/invalid/count changes,
per-row failures, unstable IDs/quantities, malformed links, zero/u32 boundaries,
64-slot bounds, family/combat gates, preserved independent rows and no actions.

Actual stdio MCP checks strict Python/Rust parity for 29 malformed section/row
cases plus non-Forever payload rejection, missing/empty/mixed/maximum observations,
exact inspection, duplicate-ID u64 totals (64 × u32-max), explicit false ownership/
lootability/action flags, private invalid-field redaction and immutable source
bytes. Expanded assertions confirm displayed loot does not affect item-search or
recipe stock; the new test passed again in 0.902s.

During verification, running Cargo test compilation concurrently with stdio held
the executable and produced Windows access-denied on relink. Both checks ended;
they were rerun sequentially and passed. A test initially rejected the pre-existing
owner identity in `character_summary`; that assertion was corrected to test new
loot/invalid-field privacy rather than remove the established owner summary.

## Development package

Built version `0.43.0-dev`, Forever/live/interface 16001 in
`dist/forever-loot-0.43.0-dev`. Archive SHA-256:
`53199b8e3b3c80474595e6a00f03a6041ec8d680ad97e93d4990644a448ed5e6`.
The four-file capture-only package passed exact manifest/source-byte checking and
the packaged Lua 5.1 export round trip. Forever capture fixture and changed-
interface refusal were true; live qualification false. The package is uninstalled.

Commands: `tools/package_addon.py --family forever --variant live --interface 16001
--version 0.43.0-dev --output dist/forever-loot-0.43.0-dev`, then
`cargo build --locked --offline --example verify_addon` and
`tools/verify_addon_packages.py --directory dist/forever-loot-0.43.0-dev
--verifier target/debug/examples/verify_addon.exe` under the Python environment.
An initial `--bin verify_addon` invocation correctly failed (this is an example,
not a bin target); no source change was needed.

## Independent review and importer regression

Round 1: two independent `gpt-6-astra` high-effort Callers and Failure/security
lenses ran all three prebuilt loot tests and the actual-MCP loot test (0.465s /
0.619s respectively). Callers also ran the bank empty/unavailable regression.
Neither found a defect in the loot section contract, source/export registration,
bounded totals or exclusion from owned inventory, recipe stock and owned gear.

Failure/security independently rejected 33 malformed slot keys/status/numeric
cases through Python validation and actual Rust stdio, plus path traversal.
Callers independently rejected nine noncanonical slot keys and checked unavailable
reason readback. Both checked archive/manifest hashes, four-file layout and
rendered source bytes. Their initial raw-byte comparison failed solely because
source CRLF is normalised to LF by the existing packager; comparison using the
actual `read_text().encode()` rendering passed. No package drift was found.

Callers reproduced a pre-existing generic mismatch: Python accepted a valid
captured loot section alongside `unavailable.loot_window`; Rust rejected it.
Author added `test_snapshot_unavailable` and reproduced `ValueError not raised`.
The bounded fix mirrors existing Rust optional-map, known-name, section-conflict,
nonblank-string and 256-byte UTF-8 rules. An explicit whitespace set preserves
Rust trim semantics where Python additionally treats U+001C..U+001F as spaces.

```text
C:/Python314/python.exe -m unittest test_snapshot_unavailable test_loot_window test_snapshots -v
Ran 8 tests in 0.887s — OK
```

New parity checks cover five valid metadata variants, 15 malformed variants and
the reported loot conflict, including multibyte limits and whitespace edges.
Triage advisory: `jev-1.14.0`, request
`decd9ded951de06ac84299c2f375ded8d9ac6e374bcf519e9b51d20cee1486de`,
589 input / 57 output tokens, 0.261s. Fix now 0.97, defer scoped 0.03,
not material/insufficient 0.00, confidence 0.96. Author confirmed and fixed it.
This does not claim that every unrelated legacy snapshot field has parity.

Round 2: both lenses clean. Callers ran the eight focused tests above (0.712s),
then independently checked 63 control/Unicode/UTF-8-boundary cases through Python
validation/store read and Rust MCP. Rejected ingests created no files. It also
exercised the actual `azeroth-operator.exe import-stdin` entry point: two valid
imports accepted, three invalid metadata inputs rejected without writes.
Failure/security ran the unavailable/loot tests (two passed, 0.845s) and all three
prebuilt loot tests. Its independent matrix checked 68 outcomes, including every
Unicode White_Space character, non-whitespace controls, trimmed reasons, mixed
UTF-8/emoji byte boundaries, lone surrogates, malformed types/names and section
conflicts. Both readers agreed and rejected MCP replies disclosed no private marker.
No further concrete issue; no lens was omitted. Reviewers used parent-built
binaries without building, editing, HTTP, installation, native/game operations or
SQLite. Actual agent token usage was not exposed.

The interaction map's older combined-capacity paragraph was also aligned with
ADR 039's explicit zero-item exception; no associated runtime change was made.

Final scoped advisory: `jev-1.14.0`, request
`02f145fa016359ef07effa01f50a372a3f0fb945f6f32b80672dbc1f644061b4`,
1,021 input / 52 output tokens, 0.290s. Land scoped 0.99, fix required 0.01,
insufficient evidence 0.00, confidence 0.98. These are uncalibrated advisory
judgements. Author disposition: land source/offline loot observation and importer
correction. Native behaviour/eligibility and the larger coverage goal remain open.

## Plan advisory

`jev-1.14.0`, request
`c305efaf2ebdb737c88daf527bade6eda5c9ae88bc710e7318b56b0b236ea1c6`,
599 input / 47 output tokens, 0.316s. Proceed 0.50, revise 0.44,
insufficient evidence 0.06, confidence 0.25. The weak/uncertain advisory is retained,
not treated as proof or approval. Concrete reference, strict boundaries and tests
plus independent review determine the source-only disposition.
