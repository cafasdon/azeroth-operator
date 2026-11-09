# Explicit captured reagent producers — 9 November 2026

Base source: `59bd59e`. The existing recipe review gains
`reagent_producer_choices` for bounded chain mode. Positive item/recipe mappings
must be unique by item, at most eight, and match a captured fixed output in the
recipe snapshot. The original four-argument public helper and default chain
shape are preserved. ADR 020 records the decision and used-choice semantics.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 20.29s
```

After moving the list-length guard ahead of allocation and adding the same
cheap MCP guard, the final incremental build passed in 8.43 seconds. Formatting
was applied with `cargo fmt`.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_recipe_producer_choices test_recipe_reagent_chain test_bulk_recipe_planning -v
```

Three actual local Rust stdio MCP tests passed in **1.401 seconds**, using
synthetic owner captures without a catalogue or network lookup. The new test
asserts:

- Default ambiguity blocks; null/empty lists in chain mode exactly match the
  default chain object.
- Two explicit producers for the same intermediate yield different correct
  counts and external requirements. Separate Python ledgers replay every
  consumption/output from initial plus external stock and match every projected
  touched-item balance.
- Requested and consulted mappings are reported. Owned intermediates avoid
  production and leave choices unused. Used choices include blocked attempts,
  not completed actions.
- Explicit unlearned, cyclic, unknown-yield and over-budget producers block
  without fallback to another valid producer. Unsupported, wrong-output,
  missing and duplicate mappings reject the request. A mapping beyond the first
  twenty ambiguity candidates works.
- Eight valid mappings work, including unused ones. Nine mappings, malformed
  IDs/shapes/extra fields and choices outside chain mode reject.
- Captures stay byte-for-byte unchanged, private markers stay absent and all
  ownership/yield/optimisation/craftability/game-action verification flags stay
  false.

## Independent review

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Neither found a concrete defect; no correction round was
required. Source remained unchanged throughout review.

Callers traced MCP schema/guards, the new helper and existing producer checks,
and ran:

```text
python -m unittest discover -s tests -p test_recipe_producer_choices.py -v
Ran 1 test in 0.361s — OK
python -m unittest discover -s tests -p test_recipe_reagent_chain.py -v
Ran 1 test in 0.383s — OK
python -m unittest discover -s tests -p test_bulk_recipe_planning.py -v
Ran 1 test in 0.647s — OK
python -m unittest discover -s tests -p test_recipe_dependencies.py -v
Ran 1 test in 0.361s — OK
```

Its independent actual-MCP probes accepted a fresh compatible inventory and
rejected stale, different-build and different-character inventories; a producer
available only in the inventory capture; missing exact root; incompatible modes;
and six additional malformed ID/list shapes. A chosen unlearned producer blocked
despite a valid alternate. Null choices in ordinary mode remained ordinary
review without a chain. Live tool schema exposed the nullable array, required
positive IDs and rejection of extra fields.

Failure/security passed the new test in 0.361 seconds and the original chain
test in 0.409 seconds. Independent probes returned:

```text
PASS: 23 malformed/bound/duplicate/invalid-unused requests rejected;
unknown-yield choice ignored when stock covers demand;
private owner mismatch rejected; explicit 9-step chain blocked.
PASS: seed=4521, 100 synthetic shared-stock cases;
completed=100, blocked=0; sequential ledger replay conserved every reported quantity.
```

Those probes covered both ID fields with negative/zero/above-u32/fractional/
Boolean/string/null/object/array values and duplicate items with equal/different
recipes. Snapshot bytes remained unchanged and private markers were absent.
Neither reviewer edited, built, mutated Git, made external calls, accessed the
game, installed or deployed anything. No SQLite was opened.

The choice is a captured recipe preference, not proof of learning, eligibility,
minimum yield or the best route. Complete owned inventory, current availability,
external-source coverage and native game qualification remain unfinished. No
live game test, installation, deployment, data sync or CI was performed.

Scoped Jev advisory: `jev-1.14.0`, request
`1d4d72c6b364fbe35e8aaffdd31705320c2c730419dd1131bf25a341354a4d71`,
877 input / 54 output tokens, 0.336 seconds; land-scoped 0.95, fix-first 0.03,
insufficient-evidence 0.02, confidence 0.92. Advisory only, not proof or approval.
