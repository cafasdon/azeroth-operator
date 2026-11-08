# Passive mail-attachment observations — 8 November 2026

The [Forever mail UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_MailFrame/MailFrame.lua)
uses loaded/total inbox counts, 16 receive attachment positions and
`HasInboxItem`/`GetInboxItem`; the latter distinguishes currency returns.
It clears `inboxBeingChecked` after an inbox update. The
[mail API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MailInfoDocumentation.lua)
defines the pending-command observation. Source evidence is not native-build
qualification.

The `mail_attachments` section reads only a visible idle inbox. Both frame
visibility results must be true; `inboxBeingChecked` and `IsCommandPending`
must be explicitly false. Missing/unknown/restricted values fail closed.
It performs two full reads and rejects changed counts, occupied slots, kinds,
IDs or quantities, rechecking idle visibility around each read.

Only item/currency IDs and quantities are exported. The item name return is
ignored, even when restricted. Header, subject, sender, body, invoice and mail
money APIs are never called. No refresh, open-message, retrieval, deletion,
sending, filter or UI-selection operation is invoked. COD and GM eligibility
are not inspected, so these are pending observations, not available stock.
Item links and modifiers are outside this contract.

The Forever-only strict schema has exact coverage `loaded_inbox_attachments`,
loaded message count 0–100, total count at least loaded and at most unsigned
32-bit maximum, and dense numbered message maps. Attachment slots 1–16 are
sparse maps. Each known row has `kind: item` plus positive-u32 `item_id`/`count`,
or `kind: currency` plus positive-u32 `currency_id`/`count`. An unresolved
occupied slot contains only `kind: unknown`. Unknown fields are rejected.
No message identifiers or stable identities are inferred from positions.

Summary counts retain loaded/total counts and observed item/currency/unknown
slot totals. Empty loaded data remains a valid partial observation even when
total messages is nonzero. Missing sections remain null. Existing item finding,
owned gear and recipe-stock calculations exclude mail. A record cannot prove
retrievability, possession or what remains in unloaded messages.

Offline verification:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test mail_attachments` passed one Lua-export/
  Rust-import test, including ignored restricted names, unknown details,
  currency distinction, visibility/idle checks, changed counts/data, numeric
  bounds, empty/unloaded states and forbidden action/header/body API stubs.
- With `PYTHONPATH=.;tests`, `python -m unittest test_mail_attachments test_mcp test_snapshots test_packaging`
  passed 13 tests, including real stdio inspection, summary unknowns, strict
  schema rejection and exclusion from `find_snapshot_item` quantities.
- Formatting and diff checks passed.
- Development archive: `local-data/mail-qualification/AzerothOperator-0.0.0-mail-test-forever-live.zip`.
  SHA-256 `42f94444222a790661939af432298a1bb467f9e9e410d748d7ff7ec7ff135c81`.

The archive was not installed. Live game testing remains deferred.

Independent reader/caller review (gpt-6.1-sol, high effort) found no concrete
flaw. `PYTHONPATH=. python -m unittest discover -s tests -p test_mail_attachments.py -v`
passed one test in 0.238 seconds. A separate 56-case real stdio/Python validator
matrix agreed on bounds, malformed types/maps, private fields, canonical and
sparse indices, family restriction and maximum 100x16 coverage/u32 values.
Cross-owner-store IDs were rejected. A synthetic bag count of three remained
three despite two pending mail items; item lookup retained only the bag location.
These supplemental fixtures were transient and made no game/source changes.

Jev 1.14.0 advisory scope assessment returned supported (request
`f61f4bfb3388c910aa4cb452293385a60d56f1f40a8bdcd77eb3fb15db4b2f6e`,
602 input / 46 output tokens, 0.327 seconds), separately from runtime evidence.

Independent security/data and capture review (gpt-6.1-sol, high effort) found
no concrete flaw. `cargo test --offline --test mail_attachments -- --nocapture`
passed one test. A temporary Rust/mlua harness loaded the actual collector and
capture path; 14 additional probes passed: missing kind preserved unknown;
secret presence/count, invalid kind/ID/count and overflow rejected; maximum
loaded and total counts accepted; added/removed second-read slots and end-of-
read hidden/pending states rejected. Every probe checked omission of the ignored
private item name. Static caller inspection found no header/body/link/GUID or
mutation calls and no stock aggregation of mail. The temporary harness was
removed. Native qualification remains pending.
