# Retained gear input verification — 9 November 2026

Scope: offline source, synthetic inventory and local stdio MCP. No native game,
simulation execution, installation, deployment, external sync or CI. Full owned
BiS, global simulation ranking and the broad feature objective remain unfinished.

New controller flag `--retain-equipment-specs` persists canonical numeric gear
material behind each resolved draft's hash. The default does not add material to
plan rows. The existing source/input binding includes this flag, rejecting a
changed retention mode before recollection. Journal status reports retention,
or null for an empty journal. All nonempty rows/windows must agree on presence.

The merger validates exact 17-slot numeric specifications against their stored
hashes and includes them in grouped draft output when retained. It rejects
unknown/private keys, booleans, null material, incomplete items, signed integer
overruns, hash changes and partial retention. This pins the current local MCP
producer shape rather than accepting arbitrary simulator protobuf fields.

Explicit export command:

```powershell
python -m tools.journal_forever_gear_windows --journal PATH --window-offset N --include-equipment-specs
```

Export returns only numeric review/hash/spec fields for the exact saved window,
and no raw selections. The default inspection omits material. Old hash-only
reports return `equipment_specs_available: false` and null material when asked;
an absent payload is never reconstructed. The flag without an exact offset
rejects. This checks one row, as the existing inspection contract specifies.

The real-module test collects baseline and one carried-ring alternative across
separate invocations, rejects an intervening retention-mode change with unchanged
database bytes, then exports the saved item-4549 ring input. Simulation and BiS
flags remain false. Synthetic tests exercise malformed material and legacy
absence with byte-identical transactional rejection. Retained numeric inputs
alone do not satisfy character/native/encounter/engine guards (ADR 011).

Author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_merge_gear_windows test_gear_coordinate_outcomes -q
5 tests passed in 31.738s
python -m unittest test_retained_gear_inputs -q
2 tests passed in 10.678s
python -m unittest test_retained_gear_inputs test_gear_resolution_accounting test_gear_window_journal -q
10 tests passed in 33.990s
```

The unchanged-default collection and model-guard regressions also passed:

```text
python -m unittest test_collect_gear_windows test_simulator_level -q
7 tests passed in 44.141s
python -m tools.journal_forever_gear_windows --help
git diff --check
```

Help/whitespace checks passed. Independent round one used Callers and
Failure/security reviews configured as `gpt-6-astra`, high effort. Per-review
token usage was not exposed. Both were clean; no correction round was needed.

Callers ran with `PYTHONPATH=tests;.`, `PYTHONDONTWRITEBYTECODE=1` and TEMP/TMP
under `C:/Temp`:

```text
python -m unittest test_retained_gear_inputs test_merge_gear_windows test_gear_coordinate_outcomes -v
7 tests passed in 44.183s; no skips
```

It traced controller/ranker/journal entry points, the unchanged seven-tuple and
draft tuple, legacy formats, opt-in paths and numeric limits against the current
Rust producer. Failure/security ran:

```text
python -B -m unittest discover -s tests -p test_retained_gear_inputs.py -v
2 tests passed in 9.945s
```

Independent bounded temporary Python probes returned:

```text
PASS 26 recomputed-hash malformed specs; mixed rows/windows; payload rollback; idempotency; exact-window read-only opt-in; private selection redaction; false qualification flags; numeric upper/lower bounds accepted
PASS duplicate keys, NaN and float overflow rejected
```

Malformed cases recomputed hashes so they exercised the numeric schema guards,
not merely digest mismatch. Failed appends and read-only inspection preserved
database bytes. Neither reviewer edited source, built, ran simulations or touched
the game/deployment. Their own synthetic temporary data was cleaned; protected
old directories stayed untouched.

Scoped advisory: `jev-1.14.0`, request
`fa96ea483541a3688d4d44de60ea1b9a178ce371da1c4744cee6f1cef273acad`,
850 input / 52 output tokens, 0.381 s; land-scoped 0.96, fix-first 0.03,
insufficient evidence 0.01, confidence 0.94. Advisory only, not proof or permission.
