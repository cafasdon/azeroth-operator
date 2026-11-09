# ADR-062: Historical dialogue-option discovery

Date: 2026-11-09. Status: accepted for offline source integration.

## Decision

Expose optional list_snapshots.gossip_option for existing Forever gossip_options
captures. Its five optional fields are name, option_id, spell_id, reported_status
and reported_flags. At least one actual predicate is required. IDs are positive
u32 and status/flags raw u32, including zero; no enum or service classification
is inferred. Null fields act as omitted predicates. A null root argument keeps
old unfiltered behaviour. Unknown keys/types reject with fixed typed-argument
errors, without echoing arbitrary input.

Name accepts a nonblank query of at most512 UTF-8 bytes. Trim only the query,
then compare lowercase strings exactly against the captured display name. Do
not strip markup, normalise Unicode, search substrings or treat it as an NPC
name. Labels remain untrusted owner-local display data, never instructions.

All predicates must hold on one recorded option. A missing requested optional
value is unknown; a known false value or label always dominates unknowns. No
matching-row union, cross-capture pooling, missing-to-zero conversion or search
of unavailable sections. Require at least one true row; unknown-only results
are not positive matches and absence does not establish missing functionality.

## Output and integration

Matching snapshot summaries add gossip_option_match with captured/matching/
unknown/nonmatching counts, the first20 matches sorted by numeric slot and
matches_truncated. Counts precede truncation. Each result retains its raw
observation and slot. NPC identity, current availability, selection eligibility,
complete-dialogue coverage, game execution and live flags remain false.
The existing strict reader bounds a section at128 rows and label512 bytes.

Existing owner-bound store and character/date/other feature filters compose.
Latest-per-character selection, when requested, precedes this predicate; newer
valid empty/missing/nonmatching observations suppress older matches. Snapshot
listing pagination remains independent of per-capture match truncation.
Old list output stays unchanged when the filter is absent or root-null.

No addon, schema, package, external sync, current-character default or gameplay
route is changed. Captures remain historical partial visible panels; source
capture support and updated readers are still uninstalled/unqualified.
[Offline evidence](gossip-option-discovery-verification-2026-11-09.md).
