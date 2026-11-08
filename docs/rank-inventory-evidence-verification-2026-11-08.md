# Ranker inventory evidence — 8 November 2026

The local `tools.rank_forever_slot` report now carries owned-review evidence
through collection and output. `gear_search` includes:

- `snapshot_id`: the capture actually used, including explicit character lookup.
- `inventory_evidence`: the unfiltered saved inventory evidence from the local
  MCP, or null when an older MCP omits it.
- `requested_slots`: ordered simulator slots selected for this search.
- `candidate_filters`: requested item IDs, set, storage, required-level and
  modifier restrictions when present.
- `maximum_simultaneous_swaps`: requested swap bound, or the selected slot count.

The existing search counts, skipped candidates, storage coverage, seven-value
collection return contract and simulator input generation are unchanged. Every
owned-review reply used in collection must agree on inventory evidence. Changes
across pages, slots or the optional set lookup reject collection. Consistently
absent or null legacy evidence stays unknown, never complete.

The ranker preserves this field as metadata from the explicitly supplied trusted
local MCP executable. It does not independently authenticate or schema-validate
that metadata. Replacing that executable with one that invents a consistent
object can invent nested evidence; the outer BiS verdict still remains false.
The current Rust producer constructs the documented bounded evidence schema.

The common final report assembly sets `best_in_slot_verified: false` for plan,
fixed, adaptive, multi-encounter, weighted and non-DPS modes. The field is not a
claim about simulation ranking precision. Rankings remain conditional on captured
storage, candidate restrictions, legality/resolution evidence, search bounds and
encounter assumptions. This does not add complete owned-set optimisation or live
ownership verification. See the [owned evidence contract](owned-inventory-evidence-verification-2026-11-08.md).

## Offline verification

`PYTHONPATH=.;tests python -m unittest test_rank_inventory_evidence` passed two
tests. The first uses the real stdio MCP and existing pinned local catalogue:
source capture, two-slot collection, item/storage/swap restrictions, excluded
unknown IDs still visible in evidence, legacy-field omission, inconsistent
cross-slot evidence rejection, and an actual `--plan-only` module subprocess.
The second exercises fixed/bulk report assembly with mocked simulation boundaries
and confirms they cannot mark absolute BiS verified. No engine performance claim
comes from a mocked simulator.

The existing physical-ring draft-deduplication test passed; all five encounter
sensitivity tests passed, including weighted-output evidence assertions. No new
native capture, game action, install or deployment was performed.

Independent reviews (`gpt-6-astra`, high effort), one clean round:

- Callers ran the final seven new/encounter tests successfully in 4.018 seconds;
  a separate real draft-dedup regression passed. Actual `main()` routing with
  mocked simulator boundaries preserved evidence in plan, single, bulk, multi,
  weighted and healing output.
- Failure/security ran the two new tests and five encounter tests successfully.
  A real two-page, 51-ring capture accepted stable evidence and rejected missing
  then present, present then missing and changed second-page evidence. It found
  no blocker within the stated trusted-MCP boundary.

The author's initial added CLI test incorrectly called synchronous `main()` from
an async test; the test was corrected to a synchronous method. No product change
was needed for that harness error. Final targeted tests and diff checks passed.

Jev plan advisory: resolved `jev-1.14.0`, request
`73228d1d20637224771dc514daa1d8b4b574ea3c3fd6a81afcdf502277977f0e`,
497 input / 47 output tokens, 0.295 seconds; proceed probability 0.86. Advisory
probabilities do not establish correctness or full optimisation coverage.

Post-verification advisory: `jev-1.14.0`, request
`1f55414866590764e1cfa782cf15341a47c9ed31aec079ee1382f1bcc81f0878`,
524 input / 50 output tokens, 0.370 seconds; land-scoped probability 0.95.
Tests and independent reviews, rather than that probability, establish the
scoped offline evidence above.
