# Prepared-window aggregation — 9 November 2026

Scope: offline source, synthetic inventory, existing local stdio preparation and
read-only local SQLite aggregation. No simulator, native game, installation,
deployment, external sync or CI. The broad feature objective remains incomplete.

Command:

```powershell
python -m tools.merge_forever_prepared_windows --journal PATH PREPARED_JSON...
```

The new CLI validates prepared artifacts against their exact source rows,
requires matching complete non-gear request context and provenance labels,
groups repeated requests and returns global review references plus conditional
search coverage. It does not accept an engine or execute any request. Requests
include supplied private profile/APL data. Consistency does not authenticate
artifacts or establish native eligibility, complete models or full owned BiS.

The actual end-to-end fixture has two physical copies of ring 4549 and one slot.
Three coordinate windows are collected and retained, then prepared by separate
real module invocations against the local MCP. Their five request instances
contain only two distinct requests: the baseline and the identical ring input.
Aggregation yields two jobs with baseline references in three windows and ring
references in two, preserving all three reported coordinates without simulation.
The journal bytes stay unchanged and private filenames are absent from output.

Synthetic tests independently check stable global request order when artifacts
are reversed, source hashes, exact gear, canonical request digests, profile and
fingerprint conflicts, missing reviews, boolean review IDs/counts, noncanonical
offsets, false flags, duplicate windows and partial coverage. A reduced-budget
probe verifies source report payloads have their own quota even when the prepared
files are small; failed aggregation leaves journal bytes unchanged.

Author direct command with `PYTHONPATH=.;tests`:

```text
python -m unittest test_merge_prepared_windows -q
2 tests passed in 7.733s
```

After adding the source-payload quota regression, the same two tests passed,
7.373 seconds. No engine or native game was invoked.

Related preparation/material regressions also passed:

```text
python -m unittest test_merge_prepared_windows test_prepare_gear_window test_retained_gear_inputs -q
7 tests passed in 27.790s
python -m tools.merge_forever_prepared_windows --help
git diff --check
```

Help/whitespace checks passed. Independent round one used Callers and
Failure/security reviewers configured as `gpt-6-astra`, high effort; per-review
token usage was not exposed. Both were clean; no correction round was needed.

Callers ran `python -B -m unittest discover -s tests -p test_merge_prepared_windows.py -v`:
two tests passed, 6.803 seconds. Its independent bounded probes reported:

```text
PASS reversed arguments: references resolve to input order, baseline global_review=1, dedup=2->1, gap=8..9, unknown qualifications retained
PASS rehashed raid/player/encounter/simOptions drift, top-level snake alias, wrong snapshot and wrong scope rejected
PASS legacy hash-only source in its own legacy journal rejected
PASS real module CLI; --engine rejected (exit 2), duplicate JSON keys rejected (exit 1); journal unchanged
PASS temporary directory cleanup; no retained SQLite handles
```

Its initial legacy fixture tried mixing retained and nonretained rows; the
journal correctly rejected that setup. A separate legacy journal then exercised
the intended aggregator rejection.

Failure/security ran the same discovery module without `-B`, with
`PYTHONPATH=tests;.`: two tests passed, 7.398 seconds. Independent synthetic
PowerShell here-string probes reported:

```text
FUNCTION REJECTIONS 17 extra_request_field, extra_row_field, two_players, no_player, equipment_absent, raid_context, player_context, simOptions_context, request_type, null_encounter_context, boolean_review, uppercase_fingerprint, empty_windows, too_many_windows, per_artifact_bytes, total_artifact_bytes, expired_read
CLI REJECTIONS 6 duplicate_keys, nonfinite, overflow, deep_nesting, invalid_utf8, over_4MiB
Input and journal bytes unchanged; false qualification flags and duplicate references verified.
```

All CLI rejections returned exit one, empty stdout and generic stderr without
private input identifiers. A reviewer helper initially overexpected the JSON
parser to reject deep nesting; the actual CLI correctly rejected the parsed
value as an invalid artifact. A PowerShell quoting error in another probe was
corrected with a here-string. Neither was a source defect. Reviewers made no
source/build/Git changes, HTTP calls, simulations or native-game actions. Their
own temporary fixtures were cleaned and protected old directories untouched.

Scoped advisory: `jev-1.14.0`, request
`9ed91092c8139442fac43cb103e8759d326e3e73aece62c7bb211fdb5f2c7eae`,
884 input / 52 output tokens, 0.282 s; land-scoped 0.92, fix-first 0.04,
insufficient evidence 0.04, confidence 0.88. Advisory only, not proof or permission.
