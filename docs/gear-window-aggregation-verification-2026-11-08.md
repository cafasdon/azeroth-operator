# Gear plan-window aggregation — 8 November 2026

Baseline `b5b3cff15a1ac073704f7e01c9fbadfed96eedff`.
Source/offline synthetic verification only. No simulator, native capture,
installation, deployment or live endpoint actions.

## Contract

Windowed plan-only ranker outputs add a canonical SHA-256 for each resolved
simulator gear input. Default non-window reports retain their original shape.
The new `tools.merge_forever_gear_windows` module accepts 1–64 explicitly named
local report files, each at most 4 MiB and at most 16 MiB combined input bytes.
It reads JSON only and never executes report contents, contacts the MCP or
simulator, modifies reports, discovers other users' data or writes a journal.

Duplicate JSON keys and nonfinite constants reject. Reports must be plan-only
with false BiS claims, canonical snapshot/scope/draft hashes, valid candidate
slots/counts and swap bound, and a total matching the weighted candidate-count
product. Window counts, bounds, next offsets and complete-window flags must
agree with that total. Baseline and sequential local review references, draft
counts, equivalent/proposed counts and distinct gear counts are cross-checked.
All reports require the same snapshot, scope and stated candidate/evidence
metadata, plus the same baseline input digest.

Positive coordinate intervals must not overlap; repeated/duplicate windows
reject rather than double-counting. Missing intervals remain explicit, with the
next uncovered offset. Large total/offset coordinates use decimal strings in
the aggregate output. Repeated baseline and other identical input hashes group
into stable global reviews with original input-report index/local review
references. Filenames, raw selections, private marker fields and arbitrary
report metadata are not copied into the result. CLI failures use a fixed safe
message without input content or paths.

`reported_coordinate_coverage_complete` describes validated local producer
metadata only. It is not independent execution attestation or a signature check.
`feasible_set_coverage_verified`, `best_in_slot_verified`, `simulation_performed`
and `game_execution` remain false even for gap-free reported intervals.
Simulation rankings, uncertainty/model consistency, persistent resume and full
owned-gear qualification are still unfinished. Artifacts lacking the new gear
digests must be regenerated; old reports are not guessed into compatibility.

## Author evidence

With `PYTHONPATH=.;tests`:

`python -m unittest test_merge_gear_windows -q`

Two tests passed, 19.349 s. Synthetic checks cover complete/partial/reordered
coverage, global baseline grouping, duplicate/overlapping ranges, cross-scope
and baseline mismatch, invalid totals/counts/status/bounds/hashes/review IDs,
boolean-as-integer rejection, and no private raw-field output.

The real integration invokes the actual plan-only ranker module for three
four-coordinate windows over a nine-coordinate synthetic owner-local ring
pool, plus a whole-scope reference. Aggregated distinct gear digests exactly
match the whole-request output. It invokes the actual merge module in a separate
process, verifies its output against the library result and checks input files
byte-for-byte unchanged. Private path/extra-field markers are absent. Duplicate
JSON keys, nonfinite JSON, malformed input and an oversized file exit safely
with empty stdout and no private marker in stderr. Every subprocess is bounded.

The unchanged local MCP executable and existing pinned catalogue supply the
synthetic producer. No Rust source changed; executable behaviour does not prove
native operation or broader optimisation correctness.

Jev plan advisory: `jev-1.14.0`, request
`bf77f2a04b9d199315f8a311e7b3149c297ed0ec3bd0ad64638ae84f7c74f5c9`,
587 input / 47 output tokens, 0.283 s; proceed probability 0.93. Advisory only.

Existing window collection and inventory-evidence regressions passed: four
tests, 13.369 s, using
`python -m unittest test_gear_combination_windows test_rank_inventory_evidence -q`.
Whitespace checks and the actual module help entry point passed. No CI or
pipeline ran.

## Independent review and correction

Round one used independent `gpt-6-astra` high-effort Callers and Failure/security
lenses. Callers was clean: six merge/window/inventory tests passed, 35.221 s.
Its independent probes confirmed unchanged default plan-row shape, canonical
window digest calculation, zero coverage from end/beyond-end windows and
rejection of duplicate positive intervals. Real producer/merger modules and
documentation were inspected; no simulator or source changes.

Failure/security ran the new tests (two passed, 20.059 s), 400 deterministic
interval/overlap/gap oracles with seed 20261108, duplicate-key/nonfinite literal
checks and five scope/numeric tampering checks. It found one parser strictness
gap: an ignored field containing JSON `1e999` became Python infinity and was
accepted, although literal NaN/Infinity tokens already rejected. No private
data or stronger attestation was emitted.

The author added a valid-report CLI regression and reproduced acceptance
(return code zero), then added a finite-checking float parser. Positive/negative
exponent overflow now reject, while a finite 1.25 extra-field control remains
accepted and omitted from output. The final author/independent results are
recorded below. No count, grouping or coordinate semantics changed in the fix.

Jev triage advisory: `jev-1.14.0`, request
`646bcf3f5c2a6c07faf8a5fe622b6c2089b79695738fee6f5e55e7a4a7c7ea22`,
531 input / 46 output tokens, 0.296 s; fix probability 1.00. Advisory only.

Final author tests passed: three tests, 25.487 s. Round-two Failure/security
review was clean: three passed, 24.139 s. Independent actual-module nested-field
probes rejected positive/negative exponent overflow and the first tested value
beyond maximum finite double, while accepting maximum finite double, finite
underflow-to-zero and 1.25 controls. Rejections had empty stdout and no private
markers. Callers was omitted in round two because the narrow parser/import/test
correction reached no additional callers. Reviewers made no source/build/live
changes and executed no simulator.

Post-correction Jev advisory: `jev-1.14.0`, request
`a0b34602ab1af676559bca4519739a458b6b89f4e1e4f5dd6946d0f405a4b346`,
639 input / 49 output tokens, 0.305 s; land-scoped probability 0.98. Advisory only.
The broad objective, simulation aggregation, persistent resume and native
qualification remain unfinished.
