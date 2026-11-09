# ADR 033: preserve inventory evidence through gear evaluation

Date: 9 November 2026. Status: source/offline; no deployment.

Owned-item review already reports pre-filter storage observations and unmatched
catalogue IDs. Collection binds that evidence into saved scope, but aggregated
and journal/final outputs omit it. Computational completion should not hide the
inventory gaps that still prevent a complete owned-gear claim.

Validate the existing evidence's exact known shape, fixed scope, canonical
partition of equipment/bags/character-bank/account-bank, strict counters and
booleans, ordered bounded positive u32 IDs and truncation/count relationships.
Bound location counters to u64 and distinct IDs to the positive u32 domain;
nonzero unmatched counts require an observed section, and zero locations and
zero distinct IDs must agree.
Require ownership, complete-resolution and BiS proof flags to remain false.
Reject extra fields rather than projecting arbitrary raw metadata. Missing/null
legacy evidence remains unknown. Existing cross-window signatures still bind
identical evidence and reject mixed scope.

Retain that object in merged coverage and journal status, including restart.
Expose it directly in unified evaluation with a separate descriptive status:
unavailable, observed with gaps, or observed without reported gaps. The last
means only that no missing section/unmatched location was reported. It does not
verify native availability, complete ownership, resolution or optimality. Keep
all existing qualification flags false and computational completion unchanged.

Software fingerprints change, so collection resumes require a new bound plan
journal. Old valid journals remain readable; malformed evidence is rejected
without mutation. No DB migration, game action, deployment or external data
lookup is introduced.
