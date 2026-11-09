# PvP cap distances — offline evidence, 2026-11-09

Previous turn was progress: 71f2db6 added personal PvP observations. Worktree was
clean. Inspection showed generic compare_snapshots already reports raw PvP paths;
the missing task was points remaining across ranks, which needs total thresholds.
ADR-071 defines the implemented bounded cap-distance contract.

Pinned source is the same Camelot PVPRankFrame.lua as ADR-070, cached at
C:/Temp/codex/forever-ui-api-9dc84f0/PVPRankFrame.lua, SHA256
640a238a1bcbbbb32ba73e5256ce1f8dd366c106abb2916300faf0f7ef3340b7.
Its detail pane queries current/cap total thresholds and adds within-rank points.
No interpolation, new external database or online lookup per MCP query is used.

Author caught a new malformed-input bug before review: explicit null totals map
passed the common optional-field shape check, then reached as_object().unwrap().
The real MCP test panicked at src/schema.rs:2030 and waited for a response. Replaced
the unwrap with a fixed validation error. The existing null case now rejects
normally. Added a ten-second bound to each MCP test call. The owned stalled Python
process32348 and children were terminated as one process tree; the original exec
session then reported exit1. No replacement test was started until cleanup.

Commands with PYTHONPATH=.;tests, TEMP/TMP=C:/Temp:

```
cargo fmt --all
cargo test --locked --offline --test player_pvp_progression
8 passed, .25s
cargo build --locked --offline
python -m unittest test_player_pvp_progression
1 passed, 1.827s
```

Tests execute actual Lua5.1 Collector/Core export, Rust decoding and the ordinary
local MCP. They cover bounded/deduplicated lookups, absent/invalid/replaced optional
getter, per-rank failures, zero/u32 limits, unstable ranks/caps/totals, final guard
mutations, orphaned thresholds, strict canonical/reference map validation and
legacy output. MCP assertions include missing current threshold, positive/zero
caps, over-cap arithmetic and u64-safe maximum current total8,589,934,590.
Malformed maps, private metadata and non-Forever snapshots reject; files unchanged.

Ordinary MCP SHA256
292171318b97026a90998423141ece3b13d1d42aecab2eb70c19b47e1e927ccc.
Uninstalled development package
dist/forever-pvp-cap-distance-0.56.0-dev/AzerothOperator-0.56.0-dev-forever-live.zip
SHA256 5c64fb66988a2bd7405f3bd3f8d60edda9cf6cd2198400b4222a7ca0fbcd4e27.
Packager CRC/member-byte checks passed. Collector LF SHA256
38b835f2be858462d261589bdd2a37f6a854b293a822d7b117c815d122860e8b;
Core unchanged e3900b9ddde4a5326eb96a3f786e3c0ee0412248205dc59f742e088fb3e0de42.

Implementation writer and independent Callers/Failure reviewers use
gpt-6-astra/high; actual token usage is not exposed. No installation, deployment,
gameplay action or live qualification. Broad MCP coverage remains active.

Jev plan advisory jev-1.14.0 request
acf6ae67bad1d0d3e3060ef9cf0dc21ad9609bdcbf0fb552ec0a064c20c7adaf:
proceed .78, revise .18, insufficient .04, confidence .67; 608 input / 47 output,
.301s. Advisory only; direct source and runtime evidence control acceptance.

Both independent lenses clean. Callers ran eight Lua/Core tests and the real-MCP
Python test, plus an independent maximum-value projection. Failure ran the same
tests and independently rejected fifteen malformed maps through both readers,
then served twenty-seven exact arithmetic cases from the same MCP process.
This specifically proves malformed/null inputs no longer panic or disable the
server. Both reviewed pinned source and ADR-071; no additional fault found.

Final Jev advisory jev-1.14.0 request
6b82adcf178abb9f75912228fd3790dc95e77fcff923a1eae2b1ccc0d77e5d05:
integrate .96, revise .03, insufficient .01, confidence .94; 592 input / 50
output tokens, .316s. Author accepts source integration from direct evidence
and independent reviews, without asserting native cap or reward eligibility.
