# Saved base skill targets — offline verification, 2026-11-10

Scope: existing Forever inspect_snapshot skill-line page gains optional
uint32 target_rank. No collector, schema for saved data, execution, installation
or game test changed. ADR-076 defines the base-rank and captured-cap boundary.

Author checks:

- cargo test --lib skill_line_filter -- --nocapture: four passed.
- cargo build --bin azeroth-operator: succeeded.
- PYTHONPATH=.;tests python -m unittest test_skill_line_filters: one actual
  Rust stdio MCP integration test passed in 0.626 seconds. Includes maximum
  target, above-cap flag, bonuses, omitted response shape and invalid types.
- git diff --check: passed.

One independent adversarial round, failure lens, resolved model gpt-6-astra
at high effort. No findings. Independent real MCP probes checked zero and
uint32 maximum, extreme signed modifiers and temporary bonuses, base gap,
above-cap targets, null/omission equality, missing/empty rows, page order,
matching counts, has_more, maximum offset, malformed targets, empty filters,
extra fields and byte-identical synthetic store files. Tool schema correctly
advertises integer/null with uint32 format and minimum zero. Documentation
matched observed responses. No network/game/deployment activity occurred.
Reviewer token usage was not exposed by the collaboration tool.

Advisory shipping judgement: Jev 1.14.0, ready confidence 0.96, request
5a502a8dcb69fbbab144067e0f0fa1d78944c4a9da4ebb5699f728e9952d9ace,
528 input and 44 output tokens, 0.287 seconds. Author accepted the bounded
source feature based on direct tests and independent real-caller evidence.
Native skill capture, present training eligibility and complete skill coverage
remain unqualified. Live testing stays deferred.

## Distance ordering follow-up — 2026-11-10

Added optional ascending order_by for points_to_observed_cap and
points_to_target, with numeric skill ID ties. Existing default skill_line_id
retains response shape. Sorting follows filtering and precedes page slicing.
Target ordering requires target_rank; zero is valid. No training cost,
feasibility, current eligibility or complete capture claim is introduced.

Author: cargo test --lib skill_line_filter -- --nocapture passed five tests;
cargo build --bin azeroth-operator succeeded. Actual stdio MCP integration
python -m unittest test_skill_line_filters with PYTHONPATH=.;tests passed in
0.827 seconds initially and 0.667 seconds after the validation fix.

Independent round one: failure and callers/guard lenses, both requested on
gpt-6-astra at high effort; resolved-model/usage metadata not exposed.
Callers traced deserialization, validation, owner-store read, Forever gate and
inspection; verified schema/default equality, both sorts and paged slices,
zero-target ties, missing/null target refusal, foreign-store rejection without
private canary leakage, wrong family/section and unchanged synthetic files.
Failure lens passed 145 independent MCP oracle/boundary assertions, but found
externally tagged enum objects accepted despite the string-only schema.
The author reproduced that finding with a new failing unit regression.
A narrow field deserializer now requires a string before parsing the enum.
Unit and real MCP regressions reject all three tagged objects with a valid
target. No unrelated enum or route was changed.

Finding triage: Jev 1.14.0 selected fix, confidence 0.85, request
b8ad19d71d3e8a900bd6e998d0561e512407a804a2a41260c20f66b3a38492bd;
421 input and 46 output tokens, 0.293 seconds.

Round two: independent failure rereview clean. Actual MCP probes rejected all
three tagged objects with valid zero target and null/object/list/string payloads.
Valid string orders, numeric zero-distance ties, schema, omitted/default equality,
target dependency and unchanged synthetic snapshot bytes passed. Callers lens
was omitted: the narrow fix changes the same already-traced page field only.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.97,
request 26234055c6fe88370fe00cb511a13f86db3a46b20a192a1e9645e8502169c57a;
560 input and 44 output tokens, 0.282 seconds. Author accepted the scoped
feature based on tests and clean independent rereview. Live testing deferred.
