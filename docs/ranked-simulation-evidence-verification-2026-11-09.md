# Ranked gear simulation evidence — 9 November 2026

## Intended result

Final saved-journal leaders, including the nested owned-gear evaluation report,
retain existing per-encounter numeric simulation evidence. No extra engine work,
checkpoint schema change, game interaction, installation or deployment.

## Contract

`simulation_evidence_by_encounter` follows supplied encounter order and retains
iterations, seed, candidate and equipped-baseline request hashes, the selected
native metric, and available metric standard deviation, paired standard error
and approximate paired interval. Death retains simulated deaths and its existing
Wilson interval. OOM has no invented uncertainty. Explicit interval bases
separate candidate-minus-baseline intervals from marginal death intervals and
unavailable intervals. Aggregate ranking uncertainty and selection adjustment
remain false. A whitelist excludes private per-iteration samples and arbitrary
metadata. Ranking objective and point estimates remain unchanged.

The existing adapter computes intervals; this projection does not recalculate
them, combine encounter intervals, or compare selected finalists statistically.
The upstream death-count rounding and model limitations remain unchanged.

## Author evidence

Windows Python 3.14, existing local Rust stdio MCP, synthetic owner captures and
synthetic simulator output; no native game or live service tests.

- `python -m unittest test_run_saved_journal test_evaluate_owned_gear`:
  7 tests, 46.856 seconds, OK. Covers actual local MCP collection, multiple saved
  pages, unchanged ranking, shared budgets, resumed checkpoints, owned workflow,
  input guards and unchanged source journals. Projection assertions cover six
  metrics, private-field exclusion, missing intervals and encounter order.
- Focused latest interval-basis assertion: 1 test, 0.000 seconds, OK.

## Advisory plan

Jev 1.14.0, request
`91c38f3cb998cf95ff371aa69a27ff47a8968c92e67fe3e8eeb705936a3ce843`:
proceed .91 / revise .04 / insufficient .05; 465 input / 47 output tokens,
0.294 seconds. Advisory only; source and tests remain authoritative.

## Independent review and disposition

One round, two independent gpt-6-astra high lenses: Callers and Failure/privacy.
Neither found a concrete flaw. Callers ran four saved-journal tests (31.883s)
and two role-workflow tests (16.002s), both OK, plus real adapter projections.
Failure ran 48 real-adapter projections across six metrics, paired/unpaired,
3/200 iterations, baseline/candidate; all passed without engine calls. Both
checked the final interval labels and focused test. No live-drift lens: this
unit changes only offline report projection, with installation still deferred.
Agent token usage was not exposed by the collaboration tool.

Final advisory Jev 1.14.0 request
`13a323e7bd9798cf330ccb6f5feb4a4514091dd98fb436a5328ae86a00c03d14`:
land .71 / fix .10 / insufficient .19; 535 input / 47 output tokens,
0.271 seconds. Author disposition: land this bounded unit on the direct test and
review evidence; full owned BiS and native qualification remain unproved.
