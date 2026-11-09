# ADR 036: preserve bulk recipe cooldown evidence through planning

Date: 9 November 2026. Status: source/offline package; no installation.

Selected-recipe capture already records cooldown/charge observations, but bulk
schematic rows omit them. Carry this evidence for fixed-item rows so existing
recipe/batch preparation checks can account for saved charge limits at scale.

Use the same optional tuple consumed by Forever's schematic UI. A shared guarded
reader handles timer/daily and charge/max components independently, checking
secret status before operations. Normalise seconds upward, validate public u32
quantities and capture only when two protected reads agree. Place lookup and
getter screening inside those protected reads. API absence, lookup/call failure,
restricted/invalid components or instability never fabricates a zero cooldown.
Valid recipe materials survive optional cooldown failure. Unsupported/budget/
unavailable bulk statuses remain status-only.

Allow a nonempty typed optional `cooldown` object on fixed bulk rows, with exact
Python/Rust schema parity: bounded seconds, optional boolean daily only with
seconds, and paired charges/max with positive maximum and charges no greater.
Reject null/extra/malformed fields. Existing old rows remain valid.

Clone the bulk field into explicit-ID recipe observation before using the
existing planning logic. It remains tied to the recipe capture timestamp; newer
inventory updates only stock. A selected schematic with the same ID is separate
evidence and does not replace bulk cooldown. Search returns the saved field;
single/batch reviews derive charge checks, bounded capacity and timer estimates.
Do not simulate recharge, claim complete recipe knowledge or current craftability,
or craft anything. Installation, API availability/performance and live secret-
value behaviour require later native qualification.
