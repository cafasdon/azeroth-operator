# ADR 093: Saved merchant quotes for recipe restock

Status: accepted, source only. Native testing and deployment remain deferred.

`plan_forever_recipe_restock` accepts an optional `merchant_quote` boolean
(default false). The quote reads merchant offers and wallet copper from the
same owner-local snapshot as the recipe and inventory. It does not fetch data,
purchase items, spend money, or act in the game.

Joint targets use the combined ordered whole-recipe material ledger. Producer
output is credited once, and shared shortages are quoted once. Cyclic or
otherwise unavailable whole-recipe order returns `material_order_unavailable`,
even if the optional individual-craft search finds a witness. Single targets
use gross whole-recipe inputs. Per-craft output reuse is not credited by this
quotation, so it can overstate acquisition needs for recycling recipes.

Only captured bags offset purchases. Bank stock and pending mail funds do not.
Existing merchant arithmetic handles bundles, limited stock, extended costs,
unknown wallet balances and bounded optimisation. The quote retains explicit
unknown/unquoted amounts. Saved affordability is conditional on this capture;
it is not current availability, native eligibility, or a guarantee of a complete
acquisition plan. Missing merchant observations return an explicit status and
null budget. Omitting the option leaves the old response unchanged.

Offline validation: `cargo build --offline` and
`$env:PYTHONPATH='.;tests'; python -B -m unittest test_recipe_restock_quote_mcp`.
The SDK caller checks shared shortages, producer credit, bank exclusion, bundle
rounding, saved affordability, unknown wallet, extended costs, missing offers,
zero demand, old response shape and cyclic-order refusal.
