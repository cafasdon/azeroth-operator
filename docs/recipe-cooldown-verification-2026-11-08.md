# Saved recipe cooldown verification — 8 November 2026

The shared classifier returns `available`, `waiting` or `unknown` from the
selected recipe's captured cooldown fields. An observed charge count takes
precedence over a recharge timer. No clock advancement or live state is inferred.

`list_snapshots` accepts `recipe_cooldown_state` for Forever only. It composes
with existing recipe, character and capture filters. An unknown cooldown still
requires an observed selected recipe. `review_forever_recipe` returns the same
value as `captured_cooldown_state`; craftability and execution remain unverified.

Author verification:

- `cargo build --locked --offline` passed.
- `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_recipe_cooldown`
  passed 11 tests on Windows, including the real Rust stdio server.
- Cases include recharging with a usable charge, exhausted charges, zero/positive
  timers, unknown cooldown, missing recipe, newest-capture exclusion and the
  non-Forever rejection. Old capture timestamps do not clear a positive timer.
- `cargo fmt --check` and `git diff --check` passed.

Jev 1.14.0 scope advice supported the recorded-state wording, separately from
the runtime evidence: request `9713be7482cd45ba92596cb7e5d04ae466dac6699fade4aeac59e5b5dae07979`,
482 input / 46 output tokens, 0.301 seconds. This is uncalibrated advisory output.

No game access, addon deployment or live capture qualification was performed.

Independent failure/caller review (gpt-6.1-sol, high effort) found no actionable
issues. Additional real-stdio probes covered charge-only data, null cooldowns,
maximum timers, unlearned recipes, combined ID/output/profession filters,
pagination, variant separation, capture bounds before newest selection and
malformed enum/non-Forever rejection. Listing/review parity and the false
live/craftability/execution flags were verified.
