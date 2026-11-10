# Saved recipe restock merchant quotation: offline evidence

Scope: ADR-093. Actual local stdio MCP and synthetic owner-local captures only.
No installation, deployment, game interaction, acquisition or paid review call.

Author commands:

- `cargo build --offline`: passed, 16.41 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_quote_mcp`:
  1 passed, 0.177 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_mcp test_recipe_restock_batch_mcp test_recipe_stock_schedule_mcp test_recipe_restock_quote_mcp`:
  4 passed, 1.547 seconds.
- `git diff --check`: passed with no output after removal of trailing blank lines.

New caller cases cover combined shortages, captured producer credit, bank
exclusion, bundle rounding, saved wallet shortfall, unknown wallet, extended
costs, missing offers, zero demand, old response shape and cyclic order.

Independent callers review used gpt-6-astra at high effort. The actual SDK test
command ran quote, joint and single tests: 3 passed, 1.174 seconds. Separate
synthetic SDK probes confirmed boolean/default-false schema, omitted=false,
stock-schedule coexistence, single bag shortage 5/cost 15 despite 100 bank units,
one combined shared-reagent line with shortage 11/cost 30, and null totals with
unquoted item 1 for unrelated, sold-out and insufficient limited offers. An empty
merchant fixture failed ingestion and was replaced by the valid unrelated offer.
No caller defect was found. Review probes did not alter source or real captures.

Independent failure/security review also used gpt-6-astra at high effort. The
new SDK test passed in 0.176 seconds. Actual executable probes checked bank/mail
exclusion (quote 10, wallet shortfall 4 despite 999999 pending mail copper),
finite/unmatched/extended-cost unknown totals, absent wallet, free/alternate
bundles (costs 0 and 6), shortage 1995 outside mix bounds (unknown mix, single
offer cost 4990), gross recycling shortages, foreign-store/path traversal refusal,
malformed booleans, empty joint targets and excessive targets. Ten synthetic
capture files remained byte-identical. No failure/security defect was found.

One review round, two independent lenses, no source findings. Root judgement:
ready for source integration within the documented conservative saved-data
scope. Paid Jev did not run under the user's spending stop. Native acceptance
and deployment remain deferred, not passed.

The adapter does not implement acquisition optimisation or a quotation from the
individual-craft schedule. Single gross inputs can overstate recycling needs.
Joint quotes remain unavailable for cyclic whole-recipe orders. Conditional
capture arithmetic does not establish current offers or native eligibility.
