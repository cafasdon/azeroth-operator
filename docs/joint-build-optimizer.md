# Native joint build experiments

The evaluator connects ADR 107's finite reference matrix to the existing local
simulator. It is a real simulation feature for an explicitly supplied model
case. It is not yet a full legal-build generator or a personal BiS service.

## Supported input

`tools.joint_build_optimizer` reads one ordinary, single-link, absolute JSON
file, at most 512 KiB. Duplicate keys and additional manifest fields reject.
The exact top-level fields are `case`, `axes`, `base_request` and `references`.
`case` and `axes` use the [planner contract](joint-build-search.md), with these
additional restrictions:

- Level 60, `dps`, one fixed encounter and the qualified positional engine hash.
- Native class ID and exact native spec key, for example `3` and `hunter`.
- One axis per category, at most five axes, at most 64 total combinations.
- `encounter_sha256` is the planner's canonical JSON digest of the supplied
  encounter object. `data_manifest_sha256` is the exact LF-byte SHA-256 of
  `clients/forever/catalogue.lock.json`.

Every reference is the canonical digest of its entire `{category, value}`
object. The reference map must contain exactly the references that axes use.
Each category replaces one complete player field:

| Category | Native field |
| --- | --- |
| gear | equipment |
| talents | talentsString |
| rotation | rotation |
| timing | cooldowns |
| consumables | consumables |

`base_request` is a single-player, single-active-party `RaidSimRequest`, not a captured equipment
review or an `IndividualSimSettings` UI wrapper. The full pinned schema rejects
unknown fields, noncanonical names, scalar coercions, unknown enums and invalid
oneof combinations. Equipment must contain exactly 17 positions with
`preserveSlots:true`. An explicit actionable `TypeAPL` rotation, race, class and
talent string are necessary. Item-swap/custom database modes are excluded.
Iterations are 2–5,000 per distinct request, the complete comparison is capped
at 100,000 iterations, and the existing adapter applies a 120-second deadline.

The public fixtures under `tests/fixtures/forever-hunter-*.json` come from the
pinned MIT-licensed WoWSims source. They contain no private character capture.
Their schema descriptor and licence live under `clients/forever/`. The
descriptor manifest and checked-in Go exporter record reproducible provenance.
Python protobuf is pinned in `requirements.txt`.

## Execution and identity

```powershell
./.venv/Scripts/python.exe -B -m tools.joint_build_optimizer prepare --input "ABS_MANIFEST"
./.venv/Scripts/python.exe -B -m tools.joint_build_optimizer run --input "ABS_MANIFEST" --engine "ABS_PINNED_EXE"
./.venv/Scripts/python.exe -B -m tools.joint_build_optimizer serve --input "ABS_MANIFEST" --engine "ABS_PINNED_EXE"
```

`prepare` supplies the native `search_id` without simulation. That identity binds
the planner identity and the canonical complete base request. A change to raid
buffs, race, pet options, trial seed or trial count therefore changes the native
identity even if every axis is unchanged. Candidate identities use that native
identity. The original `planner_search_id` remains available for traceability.

MCP advertises one `evaluate_joint_build_case` tool and its fixed search ID.
The call must supply exactly `{expected_search_id: "..."}`. It cannot change the
input file, native request, executable, objective or budget. The server holds
the validated startup manifest in memory. Changes on disk need a server restart.
Stale or unknown IDs reject before simulation. This is an owner-local server,
not a remotely shared multi-user service. A successful source/stdio test is not
proof that a new server was installed in every MCP client.

The evaluator preflights every combination before any simulation. Identical
canonical requests share one native evaluation while every original coordinate
stays in the result. Optional `--cache-root "ABS_PRIVATE_DIR"` reuses the existing
bounded owner-private numeric cache. The full canonical request and engine hash
form the key. Cache entries contain no raw manifest or profile. The path must
meet the existing ownership/ACL rules. No new unbounded result store exists.

## Results and limits

The result includes declared/resolved/evaluated counts, equivalent coordinate
count, native request hashes, coordinate identities, means, standard deviations,
seeds, trial counts, cache state and ranks. Exact observed ties share a rank.
It does not echo native requests or player names. It reports completion only
when every declared coordinate has a numeric result. An invalid combination,
engine mismatch, timeout or failed run yields no successful partial ranking.
Previously completed private numeric cache entries can still support a retry.

The score is a sample estimate. The evaluator does not label a noisy winner as
statistically better, and it supplies no global-best certificate. Snapshot and
constraint hashes are prospective references, not verified ownership or legal
build rules. Schema validation does not prove talent prerequisites, item
eligibility, spell availability, effect accuracy or that a cooldown override
affects an explicit APL cast. A native engine can accept an ineffective model
input. Those qualification flags stay false.

The personal capture path still checks observed identity, level, talents,
professions, pet and equipment through its existing guards. This explicit model
experiment does not turn a hypothetical build into a captured observation or
automatically choose the active character. Level-22 pilot recommendations and
live game tests remain outside this feature.

See [the machine design](optimisation-machine.md) for the legality, larger-space,
statistical and calibration dependencies, and the dated offline verification
record for actual measurements.
