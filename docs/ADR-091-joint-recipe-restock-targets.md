# Joint recipe restock targets from one saved capture

Date: 10 November 2026. Status: source/offline only.

Extend plan_forever_recipe_restock with optional additional_targets, one to
seven exact bulk recipe IDs and positive bag targets. The primary target keeps
its selected/exact recipe choice. All recipes and stock come from the same
explicit owner-local capture. Omission/null preserves the existing single-target
response; an empty list rejects. Require distinct recipe IDs and output base
item IDs: alternative producers for one item need an explicit choice, not
silently additive or overlapping stock targets.

Independent plans can double-spend common reagents or consume another requested
output below its target. Joint mode solves conservative integer craft counts
using captured minimum output. Each item's final modelled bag quantity equals
its captured bag stock plus its own requested production, minus all requested
recipe consumption. Same-output-item reagent use reduces its own net yield.
Bank/mail output stock never reduces the initial bag-target demand.

Start with zero crafts and monotonically update the requested counts until all
targets satisfy those constraints. Keep the existing 1,000-craft limit per
recipe, checked arithmetic and at most 8,001 passes (a changed pass adds at least
one craft to the total bound of 8,000). A nonpositive minimum net yield with a
positive deficit or a count above the bound rejects the whole request. This is
the least fixed point for the specified one-output recipes/minimum-yield model,
not a globally optimal choice among all recipes, materials, sources or prices.

Reuse existing exact recipe conflict guards and per-recipe material/condition
reviews. Combined gross materials reserve the captured stock once against the
sum of needs. For active recipes with an acyclic requested-output graph, give a
deterministic producer-before-consumer order and existing minimum-output stock
ledger. The ledger credits a producer's output once between recipe steps, not
within a recipe's repeated crafts. Same-recipe recycling is not credited.

Cycles can have a finite count solution without an executable seed/order. Keep
the count arithmetic but return cyclic_or_downstream, null suggested order,
null ordered materials/checkpoint and explicit unresolved IDs. This does not
say crafting is impossible; initial stock or external seed choices can change
execution. Zero active crafts needs no dependency order.

Modelled final quantities exclude newly acquired/withdrawn stock. Acquisition
effects can change actual final quantities; the displayed modelled maximum is
not a bound after such additions. Gross/ordered reagent observations and bank
withdrawal suggestions do not execute acquisition or establish eligibility.
No new acquisition data is credited to captured ownership. Variable output,
variants, native bag placement, learning/conditions, current stock, active
identity and native craftability remain separately unverified. No game action,
new collector, public index write, network or catalogue is added.
