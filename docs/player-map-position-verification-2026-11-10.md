# Saved player map position: offline evidence

Scope: ADR-088, synthetic Lua export and owner-local stdio MCP only.
No installation, deployment or live game test.

Direct commands:

- `cargo build --offline`: passed (24.34 seconds initially, 10.70 seconds after target error redaction).
- `cargo test --test player_map_position --offline`: 2 passed, 0.34 seconds.
- `PYTHONPATH=.;tests python -m unittest test_player_map_position`: 2 passed, 0.511 seconds.
- `git diff --check`: passed.

Lua tests run real Collectors.lua and Core.lua through mlua and decode/validate
the actual export. Three endpoint/interior coordinate pairs survive; 24 invalid,
restricted, absent, changed API/family/guard/vector/map/coordinate scenarios
refuse the section. Combat export refuses. Forbidden API counters remain zero;
extra private returns never enter the export.

Python checks strict reader parity, boundaries, wrong families, unknown/extra
fields and nonfinite values. Real stdio SDK calls test raw inspection, summary,
same/different-map targets, zero/diagonal geometry, absent/unavailable status,
invalid target redaction, section/coverage guards, foreign-store/path refusal,
all malformed hash-bound records through the Rust reader and unchanged files.
Existing owner-authorised inspect metadata retains its unavailable_reason;
the new preview never copies it.

Author corrections: summary's existing large json macro reached its recursion
bound after a field insertion. The new field now attaches after the macro;
no compiler limit changed. The first Lua numeric assertion compared integer
JSON zero with float JSON zero; it now compares numeric values. A test initially
forbade existing owner-authorised unavailable_reason metadata; the assertion now
checks the new preview and all foreign/error boundaries separately.

Plan advisory before the new cost gate: Jev 1.14.0 chose implement, confidence
0.96; hash 86ccfc8fe40f64e6afbcb8bbe7390349dc4807eb5f90ea9d1f60ea12ac417867,
630 input / 45 output tokens, 0.336 seconds. Later paid review calls are not
authorised under the new spending stop. Final judgement and triage use the
main model with independent strongest-agent evidence; no further Jev call.

Independent round 1, requested gpt-6-astra/high:

- Failure lens: no actionable defect. Existing compiled Lua tests passed 2/2;
  Python reader/stdio tests passed 2/2. Independent synthetic MCP matrix passed
  23 cases, including maximum uint32 map ID, 20 malformed targets, wrong family
  and different maps. Store bytes stayed unchanged. No new injected Lua probe
  was built; the existing compiled mutation cases supplied runtime evidence.
- Callers/privacy lens: no actionable defect. Python 2 tests passed in 0.479
  seconds, compiled Lua 2 tests passed in 0.40 seconds. Read-only package
  render_entries probe returned the four Forever files; Core and Collectors
  matched source and included the new section. Owner/hash/path/schema gates,
  redacted errors, family boundaries, unknown/different maps and absence of a
  public index write path were checked.

No live-drift lens applies because no live state changed. Root's own-model final
judgement is ready for the bounded source/offline feature, based on direct
checks and clean independent review. No further Jev call ran under the spending
stop. Native capture, active/current identity and travel-route acceptance remain
unproven; the overall feature-coverage goal remains active.
