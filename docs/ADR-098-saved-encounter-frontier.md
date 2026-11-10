# Saved encounter trade-off frontier

Date: 10 November 2026. Status: source/offline only.

The saved-journal runner and owned-gear evaluator accept
`--include-encounter-frontier`, with `--encounter-frontier-offset` (default 0).
Existing `--top` (1..20) supplies the frontier page size independently of the
weighted ranking. Omission leaves old reports unchanged. Nonzero frontier offset
without the flag fails before journal or character input access.

The optional report retains gear for which no eligible saved set has an equal
or greater improvement in every requested encounter and a strictly greater
improvement in at least one. Role metrics already express improvement in the
correct direction: damage taken, deaths and time out of mana favour reductions.
No encounter weights enter dominance. Existing encounter improvement floors do
apply. Equal point-estimate vectors retain distinct gear hashes; these are not
declared equivalent items or statistically tied builds.

An incremental exact frontier consumes every eligible candidate from fully
completed pages before aggregate top selection. Dominated vectors are removed;
all seen gear vectors remain available for duplicate consistency checks. No
partial page's aggregate shortlist becomes evidence of a frontier. A result
exists only after the parent exhausts its stable saved journal with all requested
encounters complete. Incomplete work returns unknown membership/count/hash and
no frontier sets, even if the ordinary partial ranking remains useful.

Analysis retains at most 4096 distinct eligible gear hashes, including dominated
ones. Exceeding that bound or the original shared time deadline makes membership
unknown and discards partial frontier output. It does not return a truncated
frontier as complete. The ordinary simulation/collection budgets and ranking
behaviour remain unchanged. The flag starts no simulation or external query.

Known results sort by gear hash. Count and SHA-256 of the entire sorted frontier's
hash/vector pairs stay independent of output page. Unknown results have null
count/hash/cursor, distinct from a known empty eligible frontier. Numeric values
retain their original comparison precision; exact numeric ratios canonicalise
the digest so equal integer/float values share an identity without rounding
adjacent large integer values. Deadline checks bracket final sorting/hash work,
including an empty eligible pool. Up to 20
returned sets receive existing saved-loadout and numeric evidence projections.
The original journal readback occurs after this extra join, preserving stable
source checks. Parent contexts bind encounters, engine, input fingerprints and
baseline; the frontier fingerprint alone is not full simulation provenance.

Reports inherit the parent's saved-result scope, role/unit and floor. Point
estimates do not prove statistical superiority, full owned inventory, full-set
legality, native model accuracy, global frontier or best-in-slot status. This is
an offline companion report, not a new MCP route or installed capability.
Native tests and deployment remain deferred.
