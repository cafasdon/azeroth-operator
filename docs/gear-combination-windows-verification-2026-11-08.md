# Gear combination windows — 8 November 2026

Baseline `635c93a`. Source/offline verification only. No native capture,
installation, deployment or simulator execution.

## Contract

The ranker adds optional `--combination-limit` (1–1,024) and nonnegative
`--combination-offset` (default zero). A nonzero offset needs a limit; windows
need an explicit snapshot ID rather than character-to-newest resolution.
No-window collection retains the original 1,024-total rejection. Alternative
draft budgets remain at most 128; the candidate-location and 45-second
collection bounds remain. A page may still exceed its draft budget and reject;
no truncated result is emitted. A 64-coordinate page fits the default worst-case
64-alternative budget.

Unranking addresses a product directly, without visiting preceding coordinates.
Limited-swap ordering remains baseline, increasing subset size, slot-subset
order, then each subset's product order. Counts use weighted subset dynamic
programming. A page starting past the end has zero coordinates and no next page.
All collections keep the common baseline as reference, so it is repeated across
pages independently of coordinate coverage.

`gear_search.combination_window` adds version 1, scope SHA-256, offset, limit,
considered coordinate count, decimal total, next offset and
`covers_requested_combinations`. That last flag concerns only the stated
candidate enumeration; it does not qualify inventory, feasible sets, simulator
resolution, encounter models or a statistical optimum. Existing false BiS
claims remain in every report mode. The scope includes snapshot ID, ordered
options, slots/filters, swap/collapse/dual-wield settings, catalogue source,
inventory evidence and baseline. Unknown or changing catalogue provenance
rejects a window. This is not a provenance authentication scheme.

Window ranking/aggregation and a persistent resume journal are not implemented.
The execution primitive makes bounded progress through previously rejected
spaces; it does not complete the user's full owned-gear optimisation requirement.

## Author evidence

With `PYTHONPATH=.;tests`:

`python -m unittest test_gear_combination_windows test_gear_draft_dedup test_rank_inventory_evidence -q`

Five tests passed, 13.166 s. The pure enumeration oracle checks all 27 three-slot
length patterns from 1–3, full and 1/2/3/17 maximum-swap modes, and page sizes
1/2/7/1,024 against independent exhaustive `itertools` enumeration. Concatenated
windows exactly match the reference, including empty candidate groups, group
boundaries and offsets beyond the end. A direct last-coordinate check in
5^17 = 762,939,453,125 choices returns the correct tuple without prefix walking.

Actual MCP collection uses 32 distinct catalogue rings: the full product is
1,089. Windows at offsets 0, 64 and 1,088 succeed within existing budgets,
produce correct next/final metadata and preserve the same scope fingerprint.
Narrowing item filters changes that fingerprint. Invalid bounds and character
selection with windows reject before collection.

After adding the actual module subprocess invocation, direct command
`python -m unittest test_gear_combination_windows test_simulator_level -q`
passed six tests, 6.991 s. The new test invokes both the callable CLI and
`python -m tools.rank_forever_slot` in plan-only mode with synthetic owner-local
captures; outputs agree on window metadata and retain `best_in_slot_verified:
false`. No simulator is run. Existing level/model safeguards and legacy
collection evidence tests remain intact. An initial fixture unpack expected two
values from a three-value helper; the test setup was corrected without product
changes.

The unchanged stdio executable is used as the real local caller. No Rust source
changed in this increment; executable behaviour is distinct from native proof.

Jev plan advisory: `jev-1.14.0`, request
`6da5dbcc23a081ebe1a43bc3273f09b9fd49231abe22c932ce298b91c1e03c85`,
541 input / 47 output tokens, 0.308 s; proceed probability 0.70. Advisory only.

## Independent review and correction

Round one used independent `gpt-6-astra` high-effort Callers and Failure/security
lenses. Callers ran the five window/dedup/inventory tests: five passed, 13.671 s.
Its independent stdio probes confirmed unchanged whole-request rejection above
1,024, unchanged draft rejection for an oversized window, and exact default /
window result parity for a complete 65-combination single-swap scope.

Failure/security ran the two new tests (6.612 s) and probed huge offsets with
17 groups of 256 alternatives: a 1,001-digit offset returned an empty window in
0.1023 s without walking the product. Invalid CLI bounds/character selection
produced no stdout and sanitised errors. It found one reproduced contract gap:
an intercepted `catalogue_source: {}` was accepted despite unknown-provenance
rejection being required. The actual pinned server supplies a verified digest,
but the ranker guard also covers alternate/older caller responses.

The author added the interception regression, reproduced its failure, then
required a canonical 64-character lowercase-hex catalogue digest before scope
hashing. Null/empty/bad/short/uppercase/bool digests reject, as does a valid digest
changing between owned-gear replies. The corrected window/inventory checks
passed: four tests, 12.384 s; the expanded window tests including changed-digest
coverage then passed as recorded below.

Jev triage advisory: `jev-1.14.0`, request
`7cb70fe8f2898ed010cee8b1d40bd767c67b00df402d5ebfc0f34403f91c2833`,
504 input / 45 output tokens, 0.299 s; fix probability 0.99. Advisory only.

The final expanded author window tests passed: two tests, 9.373 s. Round two
Failure/security review was clean: two tests passed, 8.621 s, including actual
MCP interception and valid-source module CLI checks. Callers was omitted in
round two because the narrow source guard/test correction reached no additional
callers. No native actions, simulations or builds were performed by reviewers.

Post-correction Jev advisory: `jev-1.14.0`, request
`b8790d7092df0addebcee6792c642da510856f125eb5992df3df356b0fbbadf1`,
640 input / 49 output tokens, 0.305 s; land-scoped probability 0.99. Advisory only.
The broad objective, global window aggregation and native qualification remain
unfinished.
