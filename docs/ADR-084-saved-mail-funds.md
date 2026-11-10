# Saved mailbox amounts and COD quote review

Date: 10 November 2026. Status: source/offline only.

Add independent Forever section `mail_funds` for numeric money/COD fields in
already-loaded inbox headers. Require visible mail/inbox frames, explicit idle
checking state and no pending command. Preserve frame/getter/guard/family
identities, including inherited frame visibility methods. Read the complete
idle/visibility context twice at each boundary. These are observations, not an
uninterrupted-visibility or native mailbox-continuity guarantee. Bound loaded rows at
100 and reported total at uint32. Query each loaded slot twice; retain a row
only when both numeric pairs match. Unreadable/restricted/changing rows are
`unavailable`; an observed change of context or counts refuses the whole section.

Reported rows contain only `status`, `money_copper` and `cod_copper`. Both
amounts are nonnegative integers at most9,999,999,999,999, an ingestion bound,
not a native game limit. First four header returns and all later metadata are
discarded. No sender, subject, name, body, invoice, timestamp or attachment
identity is captured. No mail is opened, refreshed, retrieved, paid, returned,
deleted or sent. Existing combat-export refusal remains in force.

Both readers enforce exact Forever-only fields and contiguous loaded slots.
Summary keeps reported/unavailable/unloaded counts and known sums; complete
reported-cache totals stay null unless every reported message is loaded and
readable. This does not prove complete native mailbox coverage or availability.
Mailbox money never becomes wallet funds or owned inventory. COD is a captured
quote, not a debt, obligation or permission to pay. Slot positions do not prove
mail identity or continuity and are not linked to separately captured items.

`inspect_snapshot.mail_funds_review` on `mail_funds` replaces data only when
requested. Select1..100 distinct slots from1..100, or default to loaded slots.
An explicit slot outside the loaded capture is `not_captured`, never absent.
Use a nonnegative copper reserve within the per-amount ingestion bound and
pages with uint32 offset/limit1..50 (default20). Aggregate all selected slots
before paging. Only the same snapshot's wallet funds the arithmetic; pending
mail money is excluded. Known COD above spendable balance proves the selected
quote budget does not fit; unknown selected rows prevent a positive fit claim.
A reserve not covered by the wallet also fails the reserve-aware budget.
Missing wallet/source data remains unknown; known empty remains zero. Individual
shortfalls compare quotes with post-reserve balance and do not hide a separate
reserve deficit. Payment eligibility and all game actions remain unverified.

Data stays owner-local, outside public indexing. Update readers before exporter
installation. Deployment, native loading/amount semantics and live tests remain
deferred.

## Pinned source evidence

Forever UI commit
[`9dc84f0d3bbae1768ac3405b3716000bd93eda06`](https://github.com/Gethe/wow-ui-source/commit/9dc84f0d3bbae1768ac3405b3716000bd93eda06)
uses the fifth/sixth header returns as money/COD at
[MailFrame.lua:340](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_MailFrame/MailFrame.lua#L340).
Its idle-checking field is cleared at line141 and its UI consults pending
commands at line1683. SHA-256 of the exact inspected file:
`43b76f33f6dd0a46871922a6c6db874e825d73a355ad1a9216d4436b5e2222a0`.
Raw file positions were checked locally; the upstream source was not executed.
These declarations and fixtures do not establish native qualification or
Blizzard approval.
