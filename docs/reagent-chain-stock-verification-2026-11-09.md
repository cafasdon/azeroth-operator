# Reagent-chain stock preparation — 9 November 2026

Base source: `55aaaef`. Existing `review_forever_recipe` accepts optional
`reagent_chain_stock_plan` in reagent-chain mode. Complete printed steps are
revalidated against the same captures and passed to existing ordered-batch
helpers for minimum initial materials, bank-restock suggestions, material
checkpoints and nonmaterial requirements. ADR 022 defines the relative scope.
No new capture, external source or gameplay operation is introduced.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 16.94s
```

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_reagent_chain_stock test_recipe_reagent_chain test_reagent_chain_merchant test_recipe_dependencies -v
```

Four tests passed in **1.302 seconds** through the actual local Rust stdio MCP
with synthetic owner snapshots, without a catalogue or network lookup. The new
test asserts:

- A three-step chain needs seven starting raw units, with two in bags, three
  in character bank and two in account bank. Suggested withdrawals are three
  plus two; all observed storage covers the chain while bags do not. The
  all-storage checkpoint covers three steps, while three recipe steps still
  have unknown nonmaterial checks.
- One saved bank intermediate lowers raw needs to five and requires one
  starting intermediate, not the root recipe's full three intermediate units.
- One missing raw unit leaves a material checkpoint at step two. Missing bags
  preserve unknown bag coverage/withdrawals despite sufficient bank stock;
  missing banks retain null suggestions rather than invented zero stock.
- Repeated producer steps work. A separate Python prefix-deficit calculation
  matches every starting-material quantity in each completed fixture. Free
  fixed requirements need no starting stock; blocked chain preparation is null.
- A two-copper affordable external-material quote does not turn uncovered
  observed preparation stock into covered stock or verify craftability.
- Explicit compatible inventory time is retained; different-character
  inventory, wrong mode and invalid Boolean input reject. Omission/false adds
  no stock plan. Tool schema advertises the Boolean option.
- Snapshot bytes stay unchanged, private link/character markers stay absent,
  and current-stock, withdrawal/yield/craftability/live/execution verification
  flags remain false.

These are minimum starting requirements for the printed conditional order, not
a global optimum, live readiness or a successful withdrawal/craft. Full bank
availability, actual yields and complete profession progression remain open.
Live testing, installation and deployment stay deferred; no CI or data sync ran.

## Independent review

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Neither found an actionable defect; no correction round was
needed. Product source remained unchanged during review.

Callers traced mode/schema guards, step revalidation, existing helpers and
quote/choice composition. The new focused test passed in 0.262 seconds. An
independent actual-Rust-MCP Python probe used seed `20261109`, 36 synthetic
captures and 118 calls. Omitted/false values produced identical output; enabling
the flag added only nested `stock_plan`, including when producer choices and
merchant quotes were present. Every complete chain matched an independent
prefix-deficit ledger. Ten incompatible/type requests rejected and captures
remained byte-identical.

Failure/security ran:

```text
python -B -m unittest discover -s tests -p test_reagent_chain_stock.py -v
Ran 1 test in 0.333s — OK
```

Its independent actual-MCP probe generated 180 recipe DAGs with seed `91403`,
shared reagents, varying yields, observed intermediates and missing banks.
All 180 completed chains matched independent starting-prefix, coverage,
first-uncovered checkpoint and remaining-quantity calculations. Separate
boundaries confirmed exact 4,294,967,295,000-unit needs, null unknown/cyclic
chains, owned self-reagent prefix requirements, character/build rejection,
private-link projection and six invalid mode/type/count inputs. Opt-out added
no preparation field.

Both reviewers initially generated overlapping container IDs across storage
sections. The reader correctly rejected those malformed snapshots; using the
distinct bags/character-bank/account-bank containers 0/6/7 corrected the probe
fixtures before comparisons. This was not a product fault. Neither reviewer
edited source, built, mutated Git, made HTTP/game calls, installed or deployed
anything. Temporary synthetic captures were cleaned.

Scoped Jev advisory: `jev-1.14.0`, request
`10616927fa89809c9742dcf530b9d55715a57da7869ec13360a71eacfaf900a5`,
989 input / 54 output tokens, 0.314 seconds; land-scoped 0.99, fix-first 0.01,
insufficient-evidence 0.0, confidence 0.98. Advisory only, not proof or approval.
