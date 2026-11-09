# ADR-073: Explicit candidate scan budget for larger owned pools

Date: 2026-11-09. Status: offline source integration.

The ranker rejected more than 256 physical candidate locations per slot before
equivalent-copy collapse. A synthetic pool of 257 identical rings therefore failed
although only two copies are needed across both ring slots. This blocks complete
enumeration of otherwise manageable owned pools, independently of simulation work.

Add --candidate-scan-limit to rank_forever_slot, collect_forever_gear_windows and
evaluate_forever_owned_gear. Exact integer 1..4096, default 256. Validate before MCP,
cache or file execution I/O. Reject a reported pool count over the configured
budget; never scan just a prefix and present it as complete. The maximum is
explicitly bounded and opt-in. Keep page size 50 and pagination checks, full
modifier-aware equivalence keys, physical-copy constraints, deadlines, combination,
draft, checkpoint and payload limits unchanged. No automatic budget escalation.

Report candidate_scan_limit as an operational budget. It is not a candidate
filter and does not enter the pool scope or selection binding. A successful scan
always reads the same complete filtered pool; changing only this budget within
the same source version may resume the same journal. Existing source fingerprints
still reject journals from changed collectors/rankers; no cross-version migration
or stale-source compatibility is claimed.

Larger admission is not complete ownership, native eligibility, engine accuracy
or best-in-slot proof. Distinct large pools can still exceed other budgets. Fail
or remain explicitly incomplete rather than replace full-set search with a
shortlist. This change adds no game access or addon deployment.
