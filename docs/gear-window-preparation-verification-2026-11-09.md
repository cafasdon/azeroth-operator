# Gear-window request preparation — 9 November 2026

Scope: offline source, synthetic snapshots and local stdio MCP only. No native
game, engine execution, installation, deployment, external data sync or CI.
The broad feature objective remains active and full owned BiS unfinished.

Command:

```powershell
python -m tools.prepare_forever_gear_window --mcp ABSOLUTE_EXE --journal PATH --window-offset N --template TEMPLATE_JSON --encounter-lookup ENCOUNTER_JSON
```

The new module prepares all resolved reviews in one exact saved window, without
candidate enumeration. It reads and validates one retained journal row, rechecks
each saved selection through the fixed read-only MCP equipment-review route,
requires the source snapshot/offline checks and identical retained equipment,
and calls the existing adapter to apply model guards. Successful output includes
per-review gear/request hashes and full requests, source report/scope/snapshot
identity and current file fingerprints. No engine is accepted or started.

The selected row is checked, not every journal row. Existing covered-coordinate,
resolution and feasibility distinctions remain. All four simulation/game/BiS/
feasible flags are false. Prepared requests contain supplied profile/APL data
and are private local material, not a public export. They do not establish
current equip legality or a complete validated character/encounter model.

Tests collect a synthetic level-60 Hunter's baseline and carried ring into a
retained journal, invoke the real preparation module, and compare each resulting
equipment spec with the original actual MCP review. Request hashes recompute
exactly, the adapter normalises player names, and private snapshot/template name
markers and file paths do not appear in stdout/stderr. The original journal bytes
remain unchanged throughout. Failed cases include non-start offsets, invalid
timeouts, template race conflict, captured level 20, mismatched saved item ID,
and legacy absent material. Strict selection shape tests reject private keys,
booleans and invalid location kinds.

Author inspection added a consumed-byte fingerprint guard: a file that changes
after initial hashing must not be parsed as the previously fingerprinted input.
The direct regression accepts original bytes and rejects changed bytes without
relaxing expected hashes. Ordinary pre/post fingerprints are not authentication.

Initial author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_prepare_gear_window -q
2 tests passed in 16.189s
python -m unittest test_prepare_gear_window test_simulator_level test_simulator_character_identity test_simulator_professions -q
14 tests passed in 17.622s
```

After the consumed-byte guard, author command
`python -m unittest test_prepare_gear_window test_simulator_pet_level -q`
passed eight tests, 18.027 seconds. The actual module help and whitespace checks
also passed. No engine was executed.

Independent round one used Callers and Failure/security reviewers configured as
`gpt-6-astra`, high effort; per-review token usage was not exposed. Both reviewed
the updated consumed-byte guard and were clean. No reviewer finding required a
source correction round.

Callers ran:

```text
python -B -m unittest discover -s tests -p test_prepare_gear_window.py -v
3 tests passed in 17.250s
python -B -m tools.prepare_forever_gear_window --help
```

Its independent bounded real-MCP probes returned:

```text
PASS real MCP: baseline + two-hand request; only 2 review_equipment_selection calls; offhand cleared; no adapter.run; journal unchanged
PASS cancellation after native MCP initialise: CancelledError propagated; child returncode=0; journal unchanged; temp cleanup succeeded
```

Here “native MCP” means the local compiled stdio executable, not the native WoW
client. The offhand fixture used items 5183 and 2042. Its first fixture used an
absent catalogue item and correctly failed as unresolved; the corrected fixture
passed. It prohibited `adapter.run` and checked the exact tool/clearing arguments.

Failure/security ran the same three-test discovery command: 17.965 seconds, OK.
Its independent actual preparation probes rejected duplicate source/target,
unobserved locations, private selection keys, altered retained equipment with a
correctly recomputed hash, and changed snapshot bytes under the original ID.
Every rejection had empty stdout, unchanged journal bytes and the fixed message:

```text
Gear request preparation rejected: invalid, changed, unresolved or unsupported local inputs.
```

Bounded direct input probes rejected oversized JSON, duplicate keys, NaN,
boolean container IDs, negative bags and u32 overflow. A normal one-second run
succeeded and is not evidence of timeout expiry; cancellation cleanup is covered
by the separate Callers probe. A preliminary snapshot probe had its own
whitespace-dependent mutation assertion error; corrected JSON mutation then
demonstrated source rejection. No product code changed for that probe correction.
Neither reviewer edited source, built, ran engines or touched WoW/deployment;
protected old temporary directories stayed untouched.

Scoped advisory: `jev-1.14.0`, request
`fc9d8340e04a6444f942d1ab3dfd5df20354abe995a3dae3010ab4c5944301be`,
911 input / 52 output tokens, 0.300 s; land-scoped 0.93, fix-first 0.03,
insufficient evidence 0.04, confidence 0.89. Advisory only, not proof or permission.
