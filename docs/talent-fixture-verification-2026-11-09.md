# Talent roundtrip fixture repair — 2026-11-09

Previous goal turn was progress: 4e43801 added passive trade inspection. This
turn began with a clean worktree and reproduced its separately recorded addon
test failure using the actual existing test executable:

```
target/debug/deps/addon-b06feb303860f90d.exe lua_export_roundtrip_and_failed_capture_preserves_previous --exact --nocapture
FAILED: tests/addon.rs:260: no entry found for key
```

Cause: the existing retail fixture switches AO.family to Forever while returning
only activeEntry from GetNodeInfo. Forever's baseNode requires the returned ID
to equal the requested node ID. The public pinned UI reference
C:/Temp/codex/forever-ui-api-9dc84f0/SharedTraitsDocumentation.lua describes
GetNodeInfo(configID,nodeID) returning TraitNodeInfo, including its ID. Reference
revision is 9dc84f0d3bbae1768ac3405b3716000bd93eda06. Native availability stays
unqualified. The dedicated Forever fixtures already return matching node IDs.

Change only the legacy fake: accept nodeID, assert requested ID10, return it as
ID with the same activeEntry. No production code, guard, addon archive or live
configuration changed. Do not restore the old permissive collector or skip the
Forever branch to make the test green.

```
cargo test --locked --offline --test addon lua_export_roundtrip_and_failed_capture_preserves_previous -- --exact
1 passed, .03s; 1 unrelated test filtered
cargo test --locked --offline --test forever_talent_definitions incomplete_changed_or_invalid_base_graphs_fail_the_whole_section -- --exact
1 passed, .46s; 2 unrelated tests filtered
```

The second test retains rejection of wrong/secret node IDs, changed graphs,
unreadable nodes and replaced APIs. This is offline source verification, not
game testing or full talent-model qualification. Broad coverage remains active.

Jev advisory jev-1.14.0 request
429f745a8a4af7bf923932e3b4d309e87a8ced07a3629b7ad6a2094a34771cd2:
fixture_fix 1.00, runtime_fix .00, insufficient .00, confidence1.00; 473 input /
47 output tokens, .314s. Advisory only; red/green execution establishes the fix.

Independent gpt-6-astra/high Callers/test-double review clean: compared the fake
with mandatory TraitNodeInfo.ID in the pinned reference, reran both prebuilt
tests successfully and found no masking of a production fault. One lens used
because this only repairs a test double, without changing a production guard.

Final Jev advisory jev-1.14.0 request
c87cf7f81216a0b6021eaf696cd46028b736669f16a7f79eda6925a1c6839f24:
integrate .88, revise .07, insufficient .05, confidence .82; 496 input / 49
output tokens, .286s. Author accepts the fixture repair based on direct evidence.
