# Loaded mail-attachment comparison — 8 November 2026

Source and offline verification only. No deployment, installation, game
interaction or native capture. No new capture APIs or actions are added.

`compare_snapshots` accepts optional `mail_attachment_changes: true` for
Forever. It aggregates readable saved loaded-inbox attachments by kind and
ID, preserving item/currency namespaces. Each total includes known quantity
and contributing attachment-slot count. Shared keys with a changed quantity
or slot count appear in `shared_observation_changes`, with before/after
values and signed `known_quantity_delta`. Splitting one known stack into
two can produce a zero quantity delta with a changed slot count. Message
or attachment-position reordering alone does not change aggregate results.

One-sided keys appear in `observed_only_before`/`observed_only_after` with
their known totals; the missing side is not imputed as zero and no delta
is supplied. Lists sort by kind then numeric ID. Unresolved rows remain
unattributed, counted separately for each capture. Loaded and total message
counts preserve unloaded coverage. These values describe known observations
only, not complete inbox stock. Missing either section produces null; an
observed empty inbox is distinct from unavailable data. Output is omitted
unless explicitly requested.

The response retains `coverage: "loaded_inbox_attachments"` and marks message
identity, deliveries/retrievals and inventory transfers as unverified.
No senders, subjects, body text, invoices, money, COD, modifiers, message
identifiers or private unavailable reasons are added. Pending attachments
remain excluded from owned inventory, gear drafts and recipe stock.
Existing owner-root/hash/schema, character/realm/client/build and capture
ordering checks remain in force.

The validated contract bounds quantities to 100 loaded messages × 16 slots
× unsigned-32-bit quantity, at most 6,871,947,672,000. Unsigned summation
and signed 64-bit differences therefore remain exact and within range.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_mail_attachment_comparison test_mail_attachments test_pet_view_comparison
```

Build passed; three real Rust MCP stdio tests passed. New comparison cases
cover mixed item/currency IDs, duplicate stacks, quantity and slot-only
changes, numeric ordering, moved positions, unknown slots, empty/unavailable
data, opt-in behavior, maximum aggregate positive/negative deltas and client/
character guards. Additional realm and reversed-time cases were added to
the focused comparison test. Existing mail capture validation/search and
pet-view comparison paths passed alongside the new feature.

The focused comparison test passed again with explicit realm/time guards,
numeric ordering and no delta for one-sided rows. Format and whitespace
checks passed.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no actionable finding. Callers verified real advertised Boolean
schema/default, five invalid selector types, 13 hash-valid malformed stored
records rejected without private canaries, explicit false omission and
server recovery after errors. Failure review rejected 12 additional malformed
records and two time/path violations, and passed 30 independent aggregate
oracle comparisons (seed 82711) with mixed namespaces, unknowns, partial/empty
views and maximum unsigned-32-bit counts. All probes used the current local
stdio server and synthetic owner-scoped fixtures; no live operations occurred.

Jev advisory scope assessment supported offline feature completion while
leaving broad/native completion open: `jev-1.14.0`, request
`c1a61cfc679790a4b73763c8d480669dbd20d28e70e9b280ec9784e519a33ddb`,
698 input / 51 output tokens, 0.290 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
