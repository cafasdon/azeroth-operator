# Personal group context — offline evidence, 2026-11-09

Previous goal turn was progress: 4eeb328 added PvP cap planning. Initial worktree
was clean. Source discovery found no group-context collector/section. ADR-072
defines this new task-level observation without social actions or roster data.

Pinned public UI revision 9dc84f0d3bbae1768ac3405b3716000bd93eda06:

- Blizzard_GroupFinder/Mainline/LFDFrame.lua: explicit home group checks and own
  player leader lookup. Cached SHA256
  054ecd3d27f62b3be04aeb39a4062f9872a3e6426a1281f6b4fc43efbbc35d9f.
- Blizzard_GroupFinder/Mainline/LFGList.lua: home count and raid checks. SHA256
  c96b2be2684df9b78e212a8b1b4148c95d1f843a6e04350b66b13971e3a24d1e.
- Blizzard_APIDocumentationGenerated/UnitDocumentation.lua: UnitIsGroupLeader
  accepts a unit and optional partyCategory; restricted bool may be inaccessible.
  SHA256 93004ac1b260127741471b7861360cdf7536afcb18d4c1db73b7259faa43e61c.

Files are cached under C:/Temp/codex/forever-ui-api-9dc84f0. Source also inspected
PartyInfo/UnitRole docs and default-group SecureGroupHeaders usage. These were
not used to authorise social actions or substitute default categories. Instance
category native availability and category identity continuity remain unqualified.

Author commands:

```
cargo fmt --all
cargo test --locked --offline --test group_context
6 passed, .45s
cargo test --locked --offline --test addon
2 passed, .30s
cargo build --locked --offline
python -m unittest test_group_context test_player_pvp_progression
2 passed, 2.155s
```

Python used PYTHONPATH=.;tests, TEMP/TMP=C:/Temp. Lua5.1 executes actual Collector/
Core exports and Rust decoding. Tests cover explicit category arguments, two
passes, independent missing/unstable categories, secrets, malformed values,
zero/u32 counts without max40 or flag inference, optional leader failures,
missing/colliding constants, combat/family gates and final callback replacement
of required APIs, constants, restriction guards and optional leader getter.

Actual-MCP Python checks strict Rust/Python parity, absent/single/both categories,
raw contradictory flags without inferred membership, optional unknown leader,
maximum counts, unknown/private fields and unsupported family rejection. Numeric/
boolean summaries retain false current/identity/roster/eligibility flags. Store
files remain unchanged; per-call timeout10s prevents indefinite caller waits.

Ordinary MCP SHA256
e2e5ed2be0858e3b5ebae0ecf1c6b465b81ed5199d2bdaee3b2be6efe3113e80.
Uninstalled package:
dist/forever-group-context-0.57.0-dev/AzerothOperator-0.57.0-dev-forever-live.zip
SHA256 424ac99cca29a5ebc988f6fbe77d8ff7f02d01ea22634a425c9f7e29a7f67cdd.
Packager verified CRC and every member against generated/source bytes. Collector
LF SHA256 27618c3536124c6cf64d2deec5429ec635e574e276ab172f3a407ba16963a837;
Core 7d0162cac0b2aaff226e28b7b3138bede50b7db28518432dd7e9ea83c303cdfa.
No installation, deployment, real character data or game interaction.

Implementation writer and independent Callers/Failure reviewers use
gpt-6-astra/high; actual agent token usage is not exposed. Jev plan advisory
jev-1.14.0 request b870eb019cf312f980eaff8ac5bb7c0861bf529a7358fa327af0d2d1fdcff6eb:
proceed .81, revise .15, insufficient .04, confidence .72; 588 input / 47 output,
.312s. Advisory only. Broad MCP coverage remains active and incomplete.

Both independent reviews clean: each ran six group Lua/Core tests, both addon
tests and the actual-MCP Python test. Failure independently retained thirty-six
flag/count/leader combinations exactly, without inventing home/solo state, and
rejected ten private metadata fields through both readers. Files unchanged.
Both compared the cached primary source and ADR-072; no fault found.

Final Jev advisory was uncertain: jev-1.14.0 request
03af5a025bfe853d70ca6561bd389e2ed5fd7ae57857afed95e5e156844c4705:
integrate .50, insufficient .32, revise .18, confidence .25; 560 input / 50
output tokens, .300s. Not rerolled or used as proof. Author accepts bounded
source integration on actual Lua/Core/MCP checks, primary API usage and both
clean independent lenses. Native category availability remains a separate gate.
