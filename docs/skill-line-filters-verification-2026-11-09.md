# Saved skill-line filters — 2026-11-09

Baseline 60a3e11. The preceding implementation turn made progress with queried
talent simulation guards. The intervening status-only turn made no source
progress; this continuation verified the pending working tree and finished its
records. The broad MCP coverage and owned-BiS goal remains active and incomplete.
No game, account data, addon installation, hosted service or deployment used.

## Author checks

```powershell
cargo test --locked --offline --lib skill_line_filter
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_skill_line_filters test_inventory_requirements_discovery test_loot_observation_discovery
```

Three Rust tests passed (53 unrelated filtered out, compile 11.66s). Ordinary
MCP build passed in 23.97s. Three Python tests passed in 1.858s against the actual
stdio MCP with synthetic owner-local stores. Tests cover same-row rank/cap/gap,
base-rank-only modifiers, unknown categories, zero cap, missing versus empty,
latest-before-predicate selection, date/profession/inventory composition, owner
separation, numeric sorting, 512 rows, truncation, paging and maximum offset.
Thirteen malformed filters on both routes and six malformed pages reject;
private canaries do not echo and stored bytes remain unchanged.

## Independent adversarial round

Two independent read-only gpt-6-astra high reviewers, Callers and Failure/privacy,
used prebuilt binaries without source writes, builds, network or live actions.
Both ran the three prebuilt Rust tests and the one new actual-MCP Python test:
3 passed; 1 test OK. No concrete flaw found.

Callers independently checked tool schemas, null compatibility, seven mixed
inspection modes, category uncertainty, first/last/default pages, latest
selection, ownership and sorting. Failure/privacy additionally ran 281 actual
MCP requests: 16 correctly hashed malformed stored sections rejected, nine bad
values for every numeric field rejected on both routes, 97 executed predicate
oracle trials from 100 seeded candidates agreed, and all 23 mixed Boolean review flags rejected. Malformed
records included noncanonical IDs, rank above cap, wrong numeric types, invalid
categories, unknown private fields and 513 rows. No private canaries escaped;
synthetic bytes unchanged. Reviewer token usage unavailable.

## Advisory judgement

Plan Jev resolved jev-1.14.0, request
`d56cc5cab2462480cc59875ab5641d6b755ad73d75dd93f20f13a7115e884d93`:
proceed .91, revise .07, insufficient .02, confidence .86; 680 input/47 output
tokens, .378s. Advisory only; source and caller evidence determine integration.

Final advisory Jev resolved jev-1.14.0, request
`600cf8d7a290dd3d6fe1217b36b64be95939756c175dda80a28fb1798532b29e`:
integrate .86, revise .12, insufficient .02, confidence .79; 495 input/52 output
tokens, .304s. Conditional on clean documentation readback and mechanical checks.
Continuation repeated the three scoped Python tests against the current binary:
3 passed in 1.717s. `cargo fmt --check` and `git diff --check` passed.
Ordinary MCP SHA-256:
`5fb7a36e3aa16ca64d71cb924d630caca4a6b9ed0994862adcf1b55bfe398017`.

Both reviewers completed documentation readback. Failure/privacy corrected its
original reported oracle count: three generated zero-ID candidates were skipped,
so 97 checks executed from 100 candidates. The 281-request total remains correct
(16 malformed reads + 1 listing + 144 invalid arguments + 97 oracle checks + 23
mixed flags). The final Jev input used the original 100-trial report; this count
correction is retained rather than rerolling that advisory. All remaining claims
were clean; no implementation changes or new runtime concerns arose.
