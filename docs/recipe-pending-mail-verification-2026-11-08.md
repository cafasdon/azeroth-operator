# Recipe pending-mail observations — 8 November 2026

Source and offline verification only. No deployment, installation, game
interaction or native capture. No new capture API or gameplay action.

`review_forever_recipe` adds `pending_mail_material_observations` for fixed
item reagents when the selected inventory capture has `mail_attachments`.
If no separate inventory is supplied, the recipe snapshot is the stock
source. Additional recipes use the same stock capture as the primary review,
never their own mail as a fallback. The annotation reports that source's
capture timestamp, loaded/total messages, unresolved-slot count and coverage.

Only known item attachments whose IDs match fixed reagent IDs are included,
with summed known quantities and contributing slot counts. Currency IDs are
a separate namespace and cannot match item reagents. Unrelated items are
omitted. Matching and unobserved reagent IDs sort numerically. Unobserved
IDs do not receive a zero quantity; partial/unloaded/unknown attachments
prevent a full-inbox absence inference. Missing mail or unsupported recipe
requirements produce null, while observed empty mail produces a known empty
match list and unobserved reagent IDs.

`counts_toward_material_capacity` is always false. The annotation is computed
after existing stock/capacity calculations. Bags, bank quantities, shortages,
withdrawals, material/charge capacities, combined and ordered/dependency
plans, merchant purchases and craftability flags retain their original
inputs. Repeated batch observations are not an allocation or additional
units. Pointing at pending attachments cannot verify retrieval eligibility,
COD costs, ownership or current availability. Existing owner-root/hash/schema,
character/client/build and timestamp checks remain in force.

The bounded known-mail aggregation helper is now shared with mail comparison;
its kind/ID keys, unknown counts and quantity bounds are unchanged. No
correspondence, mail identity, modifiers, private error reasons or actions
are added to output.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_recipe_pending_mail test_mail_attachment_comparison test_recipe_dependencies
```

Build passed; three real Rust MCP stdio tests passed. New integration tests
compare all computed plan fields after removing only the annotation and
source-ID metadata: adding mail changes no stock, merchant or dependency
results. They verify mixed item/currency IDs, unrelated item exclusion,
duplicate known stacks, partial/unknown coverage, numerical ordering,
observed empty versus missing data, unsupported reagents, explicit inventory
timestamp/source for primary and additional recipes, independent batch
observations and wrong-character/build rejection. Shared mail comparison
and producer-first recipe planning regression tests passed.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no concrete finding. Callers re-ran the three modules and
independently verified target-output planning with maximum u32 pending count,
inventory lacking mail suppressing all recipe-own mail, implicit batch source
selection, unsupported mixed batches and older-inventory rejection. Failure
review re-ran recipe/mail validation tests and verified maximum 1,600-slot
aggregation, unavailable bags/banks retaining null capacity, all-unknown mail
inventing no ID/quantity, no source fallback, reversed time and private extra
fields rejected, and shared comparison bounds/unknown context preserved.
No live operations occurred.

Jev advisory scope assessment supported offline feature completion while
leaving broad/native completion open: `jev-1.14.0`, request
`19e63915b8fe4ade11a727dde08ffe4102c3225128600ea3649bc5e5ef9d8802`,
687 input / 51 output tokens, 0.306 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
