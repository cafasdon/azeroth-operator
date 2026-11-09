# Player trade offer — offline verification, 2026-11-09

Previous turn was progress: 74429dd added encounter floors. Current worktree was
clean at discovery. Trade had no capture section. This unit implements ADR-069;
it does not claim full trade coverage, live availability or policy approval.

Source: pinned Gethe Forever UI TradeFrame.lua at
9dc84f0d3bbae1768ac3405b3716000bd93eda06 (linked in ADR-069), opened 9 November.
Six normal slots and quantity/item-ID tuple positions informed the contract.
TradeInfoDocumentation at the same revision was also read to distinguish
restricted money functions and trade action/event surfaces; none are used.

Author commands:

```
cargo fmt --all
cargo build --locked --offline
cargo test --locked --offline --test player_trade_offer
4 passed, .20s
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
1 passed, 1 filtered, .35s
python -m unittest test_player_trade_offer test_loot_window
2 passed, 1.233s
python -m unittest test_player_trade_offer
1 passed, 1.475s (latest explicit recipe-material separation assertions)
```

Python used PYTHONPATH=.;tests and TEMP/TMP=C:/Temp. Lua 5.1 tests execute actual
Collectors/Core export and strict Rust decoding. Dedicated tests cover two full
1–6 passes, positive-u32 boundaries, unreadable/secret/unstable rows, inaccessible
unused tuple fields, hidden/closing panels, missing APIs, final callback surface
replacement, Forever/combat gates and forbidden target/action getters. Python
checks malformed-schema parity through the real local MCP, historical inspection,
summary aggregation and unchanged files. Offers do not increase find-item stock
or recipe-material capacity. Missing/unknown/mixed/maximum cases remain distinct.

Ordinary MCP SHA256:
cc34149ea29609a3bff91083c896d22041efc7e4e73dc81d6277fe759214e7c8.

Uninstalled development package:
dist/forever-player-trade-0.54.0-dev/AzerothOperator-0.54.0-dev-forever-live.zip
SHA256 92046ff87d1ecf6e156ddb986ca1f810651c07b883810a8fd3fcb6ec0d1ec1f5.
Packager verifies every archive member against generated/source bytes and CRC.
Packaged Collector LF SHA256
1e10208a84d01d145397c4a4f619bc87fe9bc51161b10215986c867df98c637d;
Core b876af2f1028d9d0a1e411b4773d07b643eb77010d2e16306325bf474987b2f0.
No install, deployment or game operation.

Implementation writer and independent Callers/Failure reviewers used
gpt-6-astra/high; actual agent token usage was not exposed. Callers found no trade
fault after four Rust tests, real-MCP parity and inventory/material caller tracing.
Failure additionally rejected 15 malformed/private payloads and tested offered-only
item999 with six maximum quantities: summary 25,769,803,770; owned bags zero and
no locations, files unchanged.

Both reviewers additionally ran the unfiltered existing addon binary and found
lua_export_roundtrip_and_failed_capture_preserves_previous fails at
tests/addon.rs:260 because talents is absent in its family-switch fixture. The
scoped bank test passes. No talent source/test changes are in this diff; no
pre-change binary comparison was performed, so pre-existing attribution is not
claimed. This separate test failure is recorded rather than calling all addon
checks green. It is outside the new trade path and does not invalidate the direct
collector/Core/MCP evidence above.

Jev plan advisory jev-1.14.0 request
58001ae2dd42f2c783212259b3e8f78a67e0851833c82bea654b8d524ae43cfc:
proceed .53, revise .32, insufficient .15, confidence .30; 535 input / 47 output,
.346s. Uncertain advice retained without reroll; source/read-only boundaries and
independent probes support the bounded implementation. Separate-failure triage
request 0bdb1a0d1c059cdcbe1a606eebf80cf9523980723a9f8deafad40e05c2d21913:
record separately .98, block .01, insufficient .01, confidence .97; 523 input /
51 output, .368s. Advisory only. Broad MCP coverage remains active and incomplete.

Final advisory jev-1.14.0 request
078409a66ed0a39b2e334d1abbc569216e6dcbf47132caf2cdb947fc87512379:
integrate .92, revise .04, insufficient .04, confidence .88; 591 input / 50 output,
.320s. Author accepts source integration on direct evidence and both independent
clean trade reviews, retaining the separate talent-test failure for follow-up.
