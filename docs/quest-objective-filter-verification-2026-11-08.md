# Quest objective filtering — 8 November 2026

Scope: offline source and synthetic local stdio MCP. No new native API, external
database request, game action, installation, deployment or CI. Broad coverage,
full owned BiS and native qualification remain unfinished.

Existing `inspect_snapshot` already searches saved quest rewards, readiness,
costs and waypoint maps. It now accepts optional `quest_objective_kind` (exact,
case-sensitive, 1–64 UTF-8 bytes), `quest_objective_finished` (boolean), and
`quest_objective_max_remaining` (unsigned 32-bit integer). They require the
Forever `quests` section without an exact `quest_id`; every supplied objective
predicate must match one objective row. No new MCP route or profile entry is
needed. Omitting the filters preserves the existing response shape.

Remaining units use captured `required - fulfilled`, saturating at zero, as the
existing workload review does. The finished flag is independent: zero remaining
does not infer completion. Unknown counts on an otherwise qualifying row leave
that row unknown. A positive witness wins over unknown sibling rows. Known
kind/finished mismatches do not contribute unknownness. Missing objective lists
are unknown; an observed empty list is a known nonmatch.

Only definite matches remain. `quest_objective_unknown_count` reports the
number of unknown quests at this filter stage, after map/difficulty/readiness
filtering and before later timer/reward/cost filters. It is null when the quest
section is unavailable. The echoed `quest_objective_filter` makes the supplied
predicates explicit. Existing projections run over the final retained quests.
Results are historical observations, not live completion, turn-in eligibility
or permission to execute quests.

The real-MCP fixture covers missing/empty lists, cross-row false conjunctions,
unknown counts, known witnesses, saturating subtraction, false finished flags,
exact kind case, readiness/reward conjunctions and item reward projection over
the retained set. It rejects wrong sections/family, conflicting exact quest IDs,
invalid types/ranges and traversal. Snapshot bytes remain unchanged.

Direct commands from the project root:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_quest_objective_filters -v
```

The offline build completed in 13.59 seconds. The first direct MCP regression
passed one test, 0.555 seconds. After adding the reward/projection conjunction,
the focused command below passed two tests, 1.620 seconds:

```powershell
python -m unittest test_quest_objective_filters test_gossip_reward_lookup -q
cargo fmt --check
git diff --check
```

Formatting and whitespace checks passed. These are source/offline results;
they do not qualify the native collector or current in-game quest state.

Round-one independent reviews used `gpt-6-astra`, high effort, Callers and
Failure/security. Per-review token usage was not exposed. Both were clean;
no source correction round was needed.

Callers reran `python -m unittest test_quest_objective_filters -v`: one passed,
0.365 seconds. Its independent stdio probe confirmed all three optional fields
in the advertised schema; all five list projections matched a separately saved
snapshot containing only the retained quest. Seven existing map/difficulty/
readiness/timer/reward/XP/cost filters conjoined, the stage unknown count remained
one and the spending-budget conflict rejected. Probe output:

```text
PASS: advertised optional schema; all five projections equal independently reduced snapshot; seven prior filters conjoin; stage unknown count=1; spending-budget conflict rejects
```

Failure/security ran:

```text
python -m unittest discover -s tests -p test_quest_objective_filters.py -v
Ran 1 test in 0.383s — OK
```

Its bounded `PowerShell @'…'@ | python -` local stdio probe returned:

```text
PASS: false dominates missing counts; missing list unknown; empty list false; positive witness wins; u32 saturation; final checklist/item projection
PASS: 8 malformed/conflicting request guards
PASS: 6 malformed objective snapshots rejected before filtering
PASS: other owner hash unavailable without identity disclosure
PASS: owner-store file names and bytes unchanged
```

The probe asserted retained IDs 7/8, unknown count two, item-118 quantity 15,
and checklist IDs 7/8. Invalid snapshots had correct content hashes, so rejection
tested schema validation. An initial reviewer assertion used the wrong checklist
key `ready`; it corrected to `ready_for_turn_in` and reran the full probe cleanly.
No source was changed for that probe correction. Neither reviewer changed
source, built, deployed or accessed the game; protected old temporary directories
were untouched. Tested executable SHA256:
`e0e61e8d5c04909654144de8e320ede16839bcb92470ce787f7c9270f79e5be8`.

Scoped advisory: `jev-1.14.0`, request
`ed1ffefddc58a20d884410b79a0d3cb66dcc1ea9ab4b14788c2742df2feba894`,
848 input / 52 output tokens, 0.305 s; land-scoped 0.96, fix-first 0.03,
insufficient evidence 0.01, confidence 0.94. Advisory only, not proof or permission.
