# Local reference locations — 9 November 2026

Baseline `1ddbdc5`. Optional provider-backed location capture, strict readers and
scoped chain acquisition integration. Source/offline only; no provider installation,
dataset redistribution, deployment or native/game operation.

## Primary source reference

Inspected pinned QuestieDB revision `c1b50fefb19c064edc436429bef73435d9ff4254`:
[public API](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/docs/api.md),
[NPC metadata](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/src/meta/npcMeta.lua),
[object metadata](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/src/meta/objectMeta.lua).
Public entity fields expose grouped spawn coordinates. Area IDs differ from UI map
IDs; percentages, optional provider phases and instance sentinels require their
own semantics. Provider phases can be synthetic, and map support entries are not
an active-map allowlist. No conversion/helper, map table, health placeholder or
visibility inference was adopted. This pins API research, not installed dataset
revision or native acceptance. Earlier ADR 025 licensing caveats remain; no
provider content is bundled by this change.

## Author checks

A scoped `gpt-6-astra` high-effort writer implemented collector/Lua fixtures; parent
owns readers, chain projection and Python MCP tests. Actual agent usage unavailable.
Initial budget fixture put 65 reagents in one recipe and correctly failed strict
decode; splitting it into two valid rows preserved the 65-item test without
relaxing the existing 64-reagent bound.

Author also found that total-point bounds did not cap arbitrarily many empty
areas. A 4,097-empty-area fixture reproduced an unwanted empty spawns object.
Adding an independent 4,096-area counter made it omit the invalid spawn projection,
preserving known existence and item references. Triage advisory: `jev-1.14.0`,
request `57e78056511cb8ad8d50e17a71374d4de50d0c1a34b412ffa6f245f01cfb4932`,
543 input / 47 output tokens, 0.295s; fix required 1.00, confidence 1.00.
Advisory is not proof; the red/green fixture establishes the regression.

```text
cargo fmt
cargo test --locked --offline --test recipe_source_locations --test recipe_item_sources
4 location tests passed, 0.47s; 1 legacy reference test passed, 0.05s
cargo build --locked --offline
Passed; initial ordinary build 11.34s, final unchanged-Rust rebuild 1.92s
C:/Python314/python.exe -m unittest test_recipe_source_locations test_recipe_item_sources test_chain_captured_sources -v
Ran 3 tests in 1.632s — OK
```

Python environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. Lua 5.1 executes actual
collector/Core export/Rust import. Fixtures cover API absence, secrets, errors,
changes, independent existence/spawn failure, input immutability, context changes,
32-entity/16-point retention, 1,024/512 retained reference IDs, 4,096 positions,
duplicate positions, sentinel/phase-zero rules and no gameplay/map API calls.
Actual stdio MCP checks 35 malformed cache/entity/point cases through both readers,
maximum IDs/phases/counts, legacy omission, unrelated-entity exclusion, budget-
omitted statuses, old source timestamps, explicit source identity guards, net leaf
quantities and byte-identical source files.

## Package and advisory

Built the `verify_addon` example offline (1.83s), then version `0.45.0-dev` for
Forever/live/interface 16001 in `dist/forever-source-locations-0.45.0-dev`.
Archive SHA-256: `f131bfbe05b02ab7b292e66155db39f2322cd779501a7eaf6cb75a90a54f3cbb`.
Four-file capture-only rendered-source/manifest verification and packaged Lua 5.1
round trip passed. Forever fixture and changed-interface refusal true; live
qualification false. Package remains uninstalled.

Plan: `jev-1.14.0`, request
`f65cc6bbeac62904e6b03923923a538e7cbfd5cd07a559d950e2a191b9cf5dd0`,
767 input / 47 output tokens, 0.301s. Proceed 0.81, revise 0.16,
insufficient evidence 0.03, confidence 0.72. Uncalibrated advisory only.

One independent review round, Callers and Failure/security, configured as
`gpt-6-astra` high effort. Both clean. Actual agent usage was not exposed.
Both ran the four prebuilt location tests and one legacy reference test.
Callers ran new location/chain-source MCP tests: two passed in 1.005s. Its
independent probes rejected seven malformed cache/key/namespace/count variants
through both readers and showed a zero-net-demand chain returning empty location
maps despite a nonempty capture cache. Failure/security ran the new MCP test and
independently rejected 47 additional malformed type/count/linkage/coordinate
mutations in Python and Rust, without private error disclosure.

Both verified archive and four member hashes, provider-code/data exclusion and
collector/Core text against current source. Initial raw-byte comparisons differed
only because the existing packager normalises CRLF to LF; rendered contents matched.
They traced capture/import/store/review boundaries and checked historical source
binding, missing/empty data and relevant-only projection. Neither found a concrete
independent-review fault, so no follow-up round was required. Reviewers used frozen
parent-built binaries without builds, edits, network, native/game operations,
installation, deployment or SQLite.

Final source/offline advisory: `jev-1.14.0`, request
`6baf952a3c347ffe0486e568bff9ed001e2fbeff1a63438f50bad8c38a3be7c0`,
1,063 input / 52 output tokens, 0.357s. Land scoped 0.97, fix required 0.02,
insufficient evidence 0.01, confidence 0.96. Uncalibrated advisory only. Author
disposition: land bounded local reference capture and chain integration; native
qualification, dataset completeness and the wider coverage goal remain open.
