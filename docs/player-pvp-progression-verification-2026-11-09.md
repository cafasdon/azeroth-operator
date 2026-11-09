# Personal PvP progression — offline evidence, 2026-11-09

Previous goal turn was progress: 7b33e2e repaired the talent fixture. Current
worktree was clean. Source discovery found no existing use of faction2800 or
GetMajorFactionProgressionInfo; new task coverage is defined in ADR-070.

Pinned source PVPRankFrame.lua at revision
9dc84f0d3bbae1768ac3405b3716000bd93eda06 was opened via the web and fetched to
C:/Temp/codex/forever-ui-api-9dc84f0/PVPRankFrame.lua for independent local review.
SHA256 640a238a1bcbbbb32ba73e5256ce1f8dd366c106abb2916300faf0f7ef3340b7.
Vanilla/HonorFrame.lua and FrameXML/PVPHonorSystem.lua were inspected first;
neither was used to infer the Camelot rank fields. No legacy lifetime-stat or
honor-XP API was substituted for the actual rank-faction source.

Author checks:

```
cargo fmt --all
cargo build --locked --offline
cargo test --locked --offline --test player_pvp_progression
5 passed, .16s
cargo test --locked --offline --test addon
2 passed, .32s
cargo build --locked --offline
python -m unittest test_player_pvp_progression test_player_trade_offer
2 passed, 1.630s
```

Python environment PYTHONPATH=.;tests, TEMP/TMP=C:/Temp. Lua5.1 tests execute
actual Collectors/Core and decode exports through Rust. Test cases cover two
fixed-faction reads, zero/max-u32, unreadable/restricted/malformed/changed required
values, independent optional omission, namespace/getter/family/guard replacement
from getters, optional guards and final guard callbacks, combat/family gates and
forbidden side effects. Python tests send valid/malformed snapshots through the
real local MCP, checking strict reader parity, generic inspection, summary
derivation, missing/max/zero/over-threshold cases, false proof flags, rejected
non-Forever captures and unchanged files. No actual character or scoreboard data.

Ordinary MCP SHA256:
1d19fdccad912762c2743d918d8e9a2d5e98df264bf0499bfc8c11ea4cc80809.

Uninstalled package:
dist/forever-pvp-progression-0.55.0-dev/AzerothOperator-0.55.0-dev-forever-live.zip
SHA256 338a05adb66444d4f7ba751a9965f3ecd7a1b7660c42eadde3f5a2522051945c.
Packager validates CRC and every member against generated/source bytes.
Collector LF SHA256
7b6470ed9fea6ca0cd55bd6bff69d5544b2f71aa22d79d9bd309d332d3653dcb;
Core e3900b9ddde4a5326eb96a3f786e3c0ee0412248205dc59f742e088fb3e0de42.
No installation, deployment, game interaction or live qualification.

Implementation writer used gpt-6-astra/high; parent owns schemas/summary/MCP
tests. Independent Callers and Failure/privacy reviews use gpt-6-astra/high.
Actual agent token usage was not exposed.

Jev advisory plan jev-1.14.0 request
b7a152e3489552402b8ac0e160b1c366bc674bda3cfe4b5c4fb78de797a44410:
proceed .84, revise .13, insufficient .03, confidence .76; 627 input / 47 output,
.281s. Advisory only. Broad coverage, fresh active-character identity and native
rank/reward eligibility remain incomplete and are not certified by this unit.

Both independent lenses clean: each ran five PvP Lua tests, both addon tests and
the actual-MCP Python projection test. Failure additionally checked eight cap
combinations, eight forbidden metadata fields, sixteen arithmetic boundaries
and three foreign/path snapshot selectors; store files unchanged. Both verified
the cached source hash and all projected field names. Removed a brittle source
line-range comment; no runtime change. Rebuilt the uninstalled package after
that comment correction; hashes above refer to the final archive.

Final Jev advisory jev-1.14.0 request
5d010e8e12e7b39f8a6f1d540c11f24279911c56e608ae783aab2ded2f34b5f7:
integrate .76, revise .17, insufficient .07, confidence .64; 552 input / 50
output tokens, .310s. Author accepts bounded source integration from direct
checks, pinned reference and both clean independent reviews.
