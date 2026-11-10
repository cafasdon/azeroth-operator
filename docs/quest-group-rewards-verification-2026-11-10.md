# Saved quest-group rewards: offline verification

10 November 2026. ADR082 defines the feature. Source/offline only; no game,
provider query, network source, installation or deployment was used.

Opt-in reward review adds per-group views and a distinct-quest union across
all matching retained finisher groups before paging. It reuses saved XP,
wallet, fixed-item, currency-cap and renown models and adds honor/choice
coverage. Group views are explicitly non-additive. Missing values remain
unknown, observed zeros remain zero, choices are not applied and no rewards
are delivered or credited. Spell rewards/exclusivity/native eligibility and
joint route/financial/XP optimisation are not established.

## Author evidence

- `cargo test --lib quest_turn_in --offline`: 12 tests passed, including strict
  default-false opt-in, page-independent union, missing/zero context, choice
  exclusion, failed-readiness planning and serialized 256 KiB review-data cap.
  Positive fixtures are validated against the real snapshot schema.
- `cargo build --offline` passed in 24.28 seconds.
- `$env:PYTHONPATH='.;tests'; python -m unittest test_quest_turn_in_review test_quest_turn_in_rewards`:
  two actual stdio MCP tests passed in 0.604 seconds. Manual expected amounts
  verify union XP 375 with one unknown reward, 30 known required copper, 26
  known reward copper, six honor with one unknown, fixed quantities 100:3 and
  101:1, currency 200 quantity five with three above saved headroom, and ten
  renown points. Choice ID900 never enters fixed totals. Filtering readiness
  leaves unique IDs1/2/7 and XP325 even when the page is beyond the last group;
  adding group rewards would produce a larger incorrect total. A specific
  ready NPC group has minimum modeled starting copper10, shortfall5 and saved
  XP threshold150. Missing wallet/XP/caps, failed quests and disabled XP retain
  existing planner unknown/refusal semantics. Missing references yield null,
  an empty match set yields zero, invalid opt-in types reject, private quest/
  currency/faction names are absent, and fixture bytes remain unchanged.
- `git diff --check` passed.

The deterministic module worker requested gpt-6.1-sol medium; root owned the
real MCP test, tool description and docs. Resolved model/usage were not exposed.
No worker review, commit, deployment, network or game operation occurred.

## Independent review

One clean round, failure and callers/security-data lenses requested gpt-6-astra
high with fresh context; resolved models/usage were not exposed.

Failure review passed both actual MCP tests in 0.691 seconds and an independent
seeded Python oracle (seed20261110) for 80 quest/readiness/giver/page cases.
It separately computed the source quest union and XP/honor sums. Zero honor/
items versus null choice metadata, oversized opt-in refusal, unchanged default/
false output and beyond-end union of64 quests/2048 item IDs all passed.

Callers/security review passed the reward MCP test in0.230 seconds and baseline
grouping in0.288 seconds. Independent schema, foreign-owner, traversal, wrong
section and coverage-conflict probes passed without private leakage. A valid
64-quest fixture with32 fixed items and32 finishers per quest rejected oversized
reward output at limit50 while baseline succeeded. A following beyond-end union
and normal request succeeded; all owner/foreign fixture bytes stayed unchanged.
Both reviewers traced actual entrypoints, owner validation, source subsets and
pure helpers. Neither found a concrete flaw. No reviewer source writes, build,
deployment, external network or game verification occurred.

Jev 1.14.0 advisory selected ready, confidence0.77, request
`40981d3e1dbc1c441348791892683eb0a61dd270e9011840dd1624bae685693d`,
680 input/44 output tokens,0.382 seconds. Author accepted the bounded feature
on exact saved-state amounts, independent deduplication/paging oracle and clean
review lenses. This is not native qualification or a combined optimal/executable
turn-in plan. Installation/deployment and live testing remain deferred.
