# ADR 008: Explicit-snapshot gear combination windows

Date: 8 November 2026. Status: source/offline implementation.

The owned-gear audit found that the existing 1,024-combination whole-request
limit and 128-alternative draft budget prevent larger full-set searches. A
restricted winner does not prove the full-set optimum. Raising all limits would
not supply bounded execution or complete inventory/model evidence.

The ranker now has opt-in coordinate windows. An explicit immutable snapshot ID
binds a window to saved observations; character-to-newest selection is not used
for windows. Offset is nonnegative and page size is 1–1,024. The existing draft
budget, candidate-location bound and collection deadline still apply per call.
Default requests retain their old limits and behaviour.

Full products use mixed-radix unranking rather than traversing the prefix.
Limited-swap products count weighted slot subsets exactly, skip whole groups,
then unrank only the relevant product coordinates. Both preserve the existing
ordering. The common baseline is retained as a comparison reference on every
page even when its coordinate is outside that page.

Versioned window metadata includes counts, offset, next offset and a scope
fingerprint over snapshot ID, ordered candidate options, requested slots/filters,
swap restriction, copy-collapse/dual-wield mode, catalogue provenance, inventory
evidence and baseline simulator input. Consistent catalogue evidence is required
throughout collection, including a canonical lowercase SHA-256 digest. The hash detects differing stated scopes; it does not
authenticate the trusted local MCP or prove inventory/model completeness.

This is an execution primitive. It does not merge window rankings, persist a
search journal, prove exhaustive feasible-set classification, validate native
equipment or establish an absolute optimum. `best_in_slot_verified` remains
false. Future cross-window aggregation needs its own coverage, duplicate,
model/encounter/seed and interruption evidence; matching enumeration hashes alone
would not prove those requirements.

The subsequent plan-artifact aggregator adds producer-calculated simulator-input
digests, bounded local JSON validation, scope/baseline checks, nonoverlapping
reported coordinate coverage and digest grouping. It does not merge simulation
rankings or replace the missing native/model/feasible-set evidence. Input hashes
remain identity references from trusted local producer artifacts, not signatures.
