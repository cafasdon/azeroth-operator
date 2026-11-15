# Offline joint build experiment windows

`prepare_build_search_window(case, axes, offset, limit, expected_search_id)`
plans exact windows over explicit candidate references. It is the first joint
iteration component of the optimiser, not a resolver, simulator or ranking tool.

## Inputs and identities

The case contains exactly:

- `client`: `family` equal to `forever`, explicit `variant`, `build`, `interface`.
- `snapshot_id`, `engine_sha256`, `data_manifest_sha256`, `encounter_sha256` and
  `constraints_sha256`: lower-case SHA-256 references.
- `character_level`, `class_id`, `specialization`, `objective`.

Objectives are `dps`, `hps`, `tps` (maximise) and `dtps`, `death`, `oom` (minimise).
This records the requested primary objective; it does not evaluate it or invent
utility/survival requirements. The explicit constraints reference binds additional
requirements for a future qualified evaluator.

Each of 1–32 axes has exactly `name`, `category` and `candidates`. Names are
unique bounded ASCII identifiers. Categories are `gear`, `talents`, `rotation`,
`timing` or `consumables`. Each pool has 1–128 distinct lower-case SHA-256 references.
Input axis and value order are significant and preserved. References can be
reused across different axes; physical-copy and combination legality are unknown.

`search_id` binds every case field, format version and the complete ordered pools.
Each candidate binds its search identity, decimal ordinal and named selection.
Response axis summaries contain counts and digests rather than repeat all pools.
The optional expected identity rejects changed-case or changed-pool resumption.

## Windows and limits

The last axis varies fastest. `start_decimal` is inclusive and `end_decimal` is
exclusive. `next_offset` is the next coordinate or null at the end. Default limit
is 16 and the maximum is 64. Offsets are canonical decimal strings, up to 80 digits.
Offset equal to total returns an empty terminal window; larger offsets fail.

At the maximum dimensions, total is 128^32 = 2^224, represented exactly as a
decimal string. Only the requested window is materialised. Nothing about a
terminal window proves earlier coordinates were planned, visited or evaluated.
The declared matrix contains only supplied references, not every possible legal
build, complete talent tree or owned item.

CLI input is at most 512 KiB and rejects duplicate object keys. The MCP receives
SDK-decoded argument dictionaries and validates exact nested fields manually;
it does not add a parser for duplicate keys in the raw SDK protocol. Its published
schema rejects extra properties. Integers reject booleans/floats and labels reject
control/non-ASCII input. Errors are fixed and do not echo the rejected payload.

The 256 KiB limit includes text-envelope escaping and margin for SDK metadata.
Long axis names and maximum-size windows can exceed it; the whole response fails
with an instruction to reduce the limit. Default-sized windows fit the largest
supported axis definitions. No partial/truncated candidate list is returned.

## Boundaries

The separate [native joint evaluator](joint-build-optimizer.md) now resolves and
scores complete small matrices. This planner remains pure and does not execute
an engine. Native experiments have their own identity that also binds the
canonical base request, and their own stricter supported-case limits.

`plan_only` is true. Verification flags are false for simulation, objective
evaluation, constraints, legality, source/model compatibility, snapshot/current
identity, complete inventory and global optimality. These are references and
prospective context, not verified live character state. No engine binary is
opened or executed. No model support or current-playing-character choice follows
from a valid case shape. Existing simulator level restrictions remain unchanged.

The process stores no history and contacts no source website. CLI mode reads
one explicit ordinary single-link file. MCP arguments select no filesystem path,
engine, SQL or callback. The local tool is owner-private, not a multi-user service.

```powershell
./.venv/Scripts/python.exe -B -m tools.build_search prepare --input "ABSOLUTE_EXPERIMENT_JSON"
./.venv/Scripts/python.exe -B -m tools.build_search serve
```

## Offline evidence — 15 November 2026

Nine tests passed, including actual CLI and stdio MCP calls. Small complete
matrices matched an independent Cartesian product with no gap or duplicate.
Candidate IDs remained stable across different window sizes. Every case/pool
binding change rejected an expected-identity resume. A synthetic non-separable
interaction example demonstrated coverage of its toy winner; it did not model
any game mechanic or establish a real build recommendation.

The 128^32 tail test returned a bounded window without full-matrix allocation.
Strict types, nested fields, duplicate keys, hashes, offsets, terminal windows,
CLI file/input limits and actual success/error transport budgets passed.
Inputs remained unchanged and pure calls succeeded with file/process/network
access blocked.

One independent review round used two Astra/high reviewers for callers/failures
and security/data/claims. Independent comparisons checked 54 variable-size windows
covering 808 coordinates and 266 windows across 100 seeded matrices. Separate
canonical calculations matched search/candidate hashes. A 64-candidate tail probe
used 735,484 bytes peak traced allocation. These are bounded planning measurements,
not combat-performance benchmarks. No medium, high or low defect was found.
Live infrastructure review did not apply.

No actual character, owner capture, engine or game was used. Live testing stays
deferred. This module does not connect candidate resolution, legality or qualified
simulation to its windows. Existing gear-only simulation/scheduling components
remain separate; they do not establish a joint character recommendation.

## Local registration and advisory record

The local `azeroth-build-search` entry enables only
`prepare_build_search_window`. Existing entries remain unchanged. Staged/saved
configuration used explicit UTF-8 bytes, an ignored private backup, semantic
comparison, exact readback and native Codex parser checks. A saved-configuration
SDK launch from an independent directory returned the 12-coordinate synthetic
matrix and accepted a bound terminal resume. It did not read a real snapshot,
resolve a candidate or start an engine. Existing app connections may need a
refresh to discover the new companion.

The planning advisory used Jev `jev-1.14.0`, request
`c399082e8ae097df5cccb21ad6f6d6444efc443949d503424e83d7c10b6e8045`,
813 input / 47 output tokens, 0.294 seconds. It selected proceed. This was advice,
not evidence of model compatibility or a winning build.

The release advisory selected ship, using the same Jev version: request
`21c30a521b3464fc7929f8cc01a82714e9c2b5f1f6ee21f416e77920d2777a83`,
863 input / 44 output tokens, 0.308 seconds. Direct and independent evidence
formed the main session's scoped release decision. The complete optimiser
objective remains unfinished.
