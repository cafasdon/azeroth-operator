# Raw player PvP flags: offline verification

10 November 2026. Source/offline only. ADR083 records the exact shape and pinned
API evidence. No native game, installation, deployment or public-index change.

The feature spans manual player-only export, strict Rust/Python storage readers,
inspection, summary, saved-capture discovery and raw comparison. Every raw
Boolean combination is preserved; missing/restricted/unreadable/changing values
never become false. There is no normalisation into safety or combat rights,
no inferred PvP event and no gameplay action or other-unit/zone query.

## Author evidence

- `cargo test --lib player_pvp_flags --offline`: four module tests passed,
  including 208 state/predicate cases (8 states x26 nonempty predicates), all
  64 transitions, missing-side nulls and invalid/family cases.
- `cargo test --test player_pvp_flags --offline`: two full Lua5.1 collector/Core
  export/strict Rust decoder tests passed in0.24 seconds. All8 combinations
  roundtrip; normal capture makes exactly6 player-only getter reads. Changing,
  restricted, nil, number/string/error, missing/replaced API, changed family/
  guard and restriction-callback context mutation refuse the section. Combat
  export and malformed global secret guard remain refused. Forbidden-action
  count stays zero; extra private returns/errors never appear in the export.
- `cargo build --offline` passed in37.89 seconds.
- `$env:PYTHONPATH='.;tests'; python -m unittest test_player_pvp_flags test_player_pvp_flag_analysis`:
  six tests passed in1.577 seconds. Storage roundtrips, exact shapes, false vs
  missing, malformed fields, Forever-only gates and mutation isolation are
  covered. Actual stdio MCP checks all8 summaries/inspections, all26 predicates,
  all64 transitions, missing-side nulls, newest-before-filter selection,
  numeric paging, default/null-filter and omitted/false-comparison behavior,
  non-Boolean refusals and foreign-owner rejection. Snapshot bytes stay unchanged.
- `git diff --check` passed.

Implementation used two deterministic workers (requested gpt-6.1-sol medium,
fresh context): Rust workflow and Python reader/tests. Root owned exporter,
Core wiring, Lua/MCP integration and documentation. Resolved models/token usage
were not exposed. Workers did not spawn review, commit, deploy, use the network
or operate a game. The Python worker additionally passed six existing snapshot
tests; those are reader regression evidence, not native qualification.

## Independent review and correction

Failure and callers/security-data lenses requested gpt-6-astra high with fresh
context; resolved models/usage were not exposed. Both found one concrete
documentation defect: the API link used parsed-page line numbers rather than
the exact pinned file's raw GitHub lines. Author reproduced the hash and raw
Name positions2225/2241/2256, then corrected ADR083 to2224–2269/#L2224. The
function signatures and restriction marker were already correct. Both lenses
independently rechecked the narrow prose fix; no runtime rerun was needed for
that correction.

Jev1.14.0 advisory triage selected fix, confidence0.98, request
`4b9248124c409d344cfea2cdb1a4c62cb7ed17d3a4a4c7726d9ed24498793d78`,
464 input/46 output tokens,0.300 seconds. Source hyperlink positions now come
from the exact pinned file, independently of transformed browser line labels.

Failure review passed six Python/MCP tests in1.186 seconds and the two existing
Lua integration test binaries. Independent actual MCP probes rejected five
character/realm/build/variant/time conflicts,21 malformed predicates and a
Classic filter. Callers/security review passed six tests in1.249 seconds and
both Lua tests, plus42 correctly hashed malformed/family/extra-field inspection
and summary refusals, three comparison gates and capabilities/tool-schema/
required-section registry checks. Synthetic bytes stayed unchanged; owner
canonicalisation and hash validation precede all outputs. Both follow-ups
were clean. No reviewer source edits, build, network, deployment or game action.

Jev1.14.0 advisory shipping judgement selected ready, confidence0.58, request
`0f86179d8b86ead99ec51025676ac1ce885986e5019a2a1618ae759a99949afa`,
703 input/44 output tokens,0.314 seconds. Author accepted only the bounded
source/offline feature on exhaustive flag/transition tests, actual MCP owner
checks and clean independent follow-ups. This advisory is not native evidence
or approval. Installed behaviour, character continuity, combat eligibility and
zone safety remain unproven. Installation/deployment/live testing stay deferred.
