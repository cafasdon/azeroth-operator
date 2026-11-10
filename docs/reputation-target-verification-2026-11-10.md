# Explicit reputation targets: offline evidence

Scope: ADR-100. Actual local stdio MCP and synthetic owner-local captures only.
No external source, real private capture, purchase, turn-in or deployment.

Author commands:

- `cargo build --offline`: initial 25.91 seconds, final runtime build 10.15 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_reputation_target_mcp test_reputation_filters`:
  2 passed, 1.890 seconds. TEMP/TMP used C:/Temp/codex for old fixtures.
- `PYTHONPATH=.;tests python -B -m unittest test_reputation_target_mcp`:
  final schema/null/signed-bound checks passed, 1.499 seconds.
- `git diff --check`: passed.

The new actual SDK test checks the new target's signed schema range and strict
redacted parser, unchanged old next-rank outputs, supplied item/copper/reserve
costs, joint capacities, overflow flags, unknown wallet/bags, known zero
bottlenecks, absent/special factions, capped-reaction uncertainty, met targets,
source privacy/foreign-store refusal and unchanged saved bytes. A 140-case
independent integer reference varies signed current/desired counters and gains;
both signed int32 extremes are also tested without wrapping. Names stay absent
from the new nested projection; the old owner-authorised raw faction view stays
unchanged.

Independent review round: callers and failure/math/privacy, each requested on
gpt-6-astra at high effort. Both clean, no source fixes needed. Calls used the
app's existing ordinary review capacity; no external inference purchase or
usage reset occurred. Private account/usage identifiers are not recorded here.

Callers ran both SDK tests: 2 passed, 2.089 seconds. Separate actual-MCP probes
confirmed unchanged target results with old bank/mail options, missing or
inconsistent standard evidence remaining unknown, seven invalid dependencies/
filter/cost combinations refused, met targets with both resources absent
(supported 0, unknown 2), and wallet-only capacity 16.

Failure/math/privacy ran the target test (1 passed, 1.314 seconds) and existing
reputation filters (1 passed, 0.355 seconds) through the existing debug binary.
An independent integer oracle ran 160 cases (seed 100110) covering both signed
limits, uint32 gains, uint64 costs, missing resources, zero bottlenecks, reserves
and overshoot. It matched totals, overflow, joint support and remaining gaps.
Two inconsistent-bracket cases, eight malformed/context/traversal/redaction
cases and a valid non-Forever refusal also passed. Initial synthetic assumptions
about reaction zero and Retail banks were corrected to valid probes, without
source or validator changes. No real captures, network or live game were used.

Root judgement: ready for source integration within the constant-gain assumption
model. Actual rates, repeatability, caps, current state and native achievability
remain unproven. Live tests and deployment remain deferred. The broad system-
coverage objective is not complete.
