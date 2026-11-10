# Battlefield queue slot observation — offline verification, 2026-11-10

Scope: ADR-078. Player-requested Forever export, capacity and allowlisted status
only; no queue actions. Both strict readers and section registries updated.
No installation, deployment or live game testing occurred.

Author checks:

- cargo test --test battlefield_queue_slots -- --nocapture: two passed.
  Actual Lua 5.1 collector/Core export through Rust import preserves stable,
  zero and 32-slot observations and all five tokens. Changing count/status,
  missing/restricted/unknown/error values and replaced family/API/guard cause
  unavailability. Secret secondary returns are discarded; zero forbidden calls.
- cargo build --bin azeroth-operator: succeeded.
- PYTHONPATH=.;tests python -m unittest test_battlefield_queue_slots: one actual
  Rust stdio MCP test passed in 0.824 seconds. Both validators reject private
  extras, wrong tokens/types/counts/slots and client families. Foreign owner
  data is inaccessible. Missing differs from zero; coverage lookup works and
  all synthetic snapshot bytes are unchanged.
- Temporary four-file development package, Forever live/interface16001,
  version0.0.0-dev SHA256
  7ea954d234d9bd6a308c59fd296ee1576ccd2544e7b3cc062a6cdbf5c882ff1c.
  Actual verify_addon passed packaged Lua5.1 roundtrip, existing Forever capture
  fixture and changed-interface refusal. Package/stage automatically removed.
  Generic package fixture does not qualify native queue APIs; the separate
  source fixture supplies the queue projection evidence.
- git diff --check: passed.

One independent adversarial round: failure and security/data plus callers
lenses, requested gpt-6-astra high; resolved model and token usage not exposed.
No concrete findings. Failure reviewer rejected 25 independent malformed,
restricted and context-changing Lua cases. Capacities 0/1/3/32 with all-none
slots retained exact capacity, twice-capacity status reads and four count reads;
secondary return sentinels were untouched. Twenty-nine Python/real-binary schema
parity cases passed. Data/callers reviewer ran the real MCP and compiled Lua
tests plus nine fresh coverage/registry/private-input MCP probes. All passed;
docs distinguish capacity from active count and saved observations from current
membership, permission, full queue coverage or native qualification. No game,
network or deployment probes were performed by reviewers.

Source reference: mirrored Forever queue UI and Camelot override linked in
ADR-078, checked 10 November. Getters/status tokens are source evidence only;
installed availability and Blizzard authorisation remain unverified.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.74,
request 319f1e915348a9d13d48c4f9e38be17de5d05a5d4d1ec2119dabfdca13e81663;
731 input and 44 output tokens, 0.308 seconds. Author accepted the bounded
source feature based on direct tests and both independent reviews. Existing
queue action exclusions and parked native/install/deployment gates remain.

## Saved status summary — 2026-11-10

Character summary now reports per-token counts and numeric slot lists, saved
capacity and a raw non-none count. Missing data remains null; zero and all-none
observations remain distinct. All current membership, coverage, identity,
acceptance, execution and live-verification flags stay false. No collector,
guard, permission or game-call path changed.

Author: cargo build --bin azeroth-operator succeeded. With PYTHONPATH=.;tests,
python -m unittest test_battlefield_queue_slots passed one actual Rust stdio
MCP integration test in 0.652 seconds, including summary values, numeric order,
missing/zero/all-none, strict schema/owner boundaries and unchanged store bytes.
Diff check passed.

One independent adversarial round, failure lens, requested gpt-6-astra high;
resolved model and token usage not exposed. No concrete finding. Independent
real-MCP probes passed 198 capacity/status/mixed/reverse-order cases across
capacities zero through 32. Missing summary was null. Seventeen malformed
on-disk captures rejected before summary while the server remained usable.
The reviewer also independently ran the actual MCP test and checked all six
boundary flags stayed false. Only temporary synthetic probes were written.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.88,
request 100d1a57f8c6083c576cb47521d78d502cdb712063e1d7cbf8bfd34192e602be;
563 input and 44 output tokens, 0.372 seconds. Author accepted the scoped
analysis change based on direct tests and clean independent review. Native
qualification, installation, deployment and live testing remain deferred.

## Historical battlefield status discovery — 2026-11-10

Added list_snapshots.battlefield_queue with required exact raw status and
optional exact slot/minimum matching count. Thresholds apply after an exact
slot restriction. Latest-per-character selection precedes the predicate;
missing/empty/nonmatching captures do not resurrect older matches. Matching
metadata retains capacity and every matching numeric slot, without asserting
current membership, identity, full coverage or acceptance. No capture, game
call, public-index input or action route changed. See ADR-078.

Author: cargo build --bin azeroth-operator succeeded. With PYTHONPATH=.;tests,
python -m unittest test_battlefield_queue_filters passed one actual Rust stdio
MCP test in 0.742 seconds. Covered all five tokens, zero/missing latest capture
suppression, explicit historical time bound, exact slots, count thresholds,
contradictory slot/count predicates, 32-slot numeric order, owner/family gates,
malformed type/token/bound/extra-field redaction, composition with inventory
and coverage predicates, null/default compatibility and paging. Synthetic
snapshot bytes remained unchanged. Diff check passed.

One independent adversarial round: failure and security/data plus callers
lenses, requested gpt-6-astra high; resolved model and token usage not exposed.
Neither found a concrete flaw. Failure reviewer passed 125 independent reference
filter cases over 66 captures plus missing data, timestamp-tie resolution before
filtering, six paging cases including maximum offset and 30 malformed/redaction
cases. Security/data review passed eleven fresh MCP probes of newer nonmatches,
nullable predicates, composition, corrupt private captures, token/type redaction
and absent slots. Both independently ran the actual MCP test. Readers remained
validated/root-contained and result lists bounded to 32 numeric slots; no new
capture, execution or public-index path was introduced. Docs matched behaviour.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.83,
request ef8ab5c614a2039885b899d9fbf4dd9b908931bffa7051aa1c9e01f8516688a5;
639 input and 44 output tokens, 0.278 seconds. Author accepted the bounded
source change based on direct tests and both independent reviews. Current
membership, complete coverage and native qualification remain unproven;
installation, deployment and live testing remain deferred.

## Historical positional battlefield comparison — 2026-11-10

Added optional battlefield_queue_changes to existing snapshot comparison.
Reported capacities stay independent, missing sections retain unknown changes,
and unrecorded slots retain null rather than none. Full changed-slot details
are numerically ordered within 32 slots. Slot identity, queue continuity and
join/leave events remain unverified. No collector, action or deployment changed.
See ADR-078 for the contract.

Author: cargo build --bin azeroth-operator succeeded. With PYTHONPATH=.;tests,
python -m unittest test_battlefield_queue_compare passed one actual Rust stdio
MCP test in 0.447 seconds. Covered all captured status kinds, changed capacity,
missing/zero observations, null unrecorded slots, full 32-slot ordering,
self-comparison, malformed/null flags with redaction, default equality and
owner/character/family/chronology rejection. Snapshot bytes remained unchanged.
Diff check passed.

One independent adversarial round: failure and security/data plus callers
lenses, requested gpt-6-astra high; resolved model and token usage not exposed.
No concrete findings. Failure review passed 180 independent positional reference
cases including all 25 token transitions, capacity boundaries, missing data,
reversed/reused positions and seeded rows. Sixteen malformed flag/time/identity
cases rejected; omitted and false flags matched. Security/data review passed
fourteen fresh MCP probes covering exact build/variant/interface/realm gates,
private corrupt rows and nested flag redaction, missing capacity, unrecorded
slot nulls, positional reorder, schema/composition and unchanged defaults.
Both reviewers independently ran the real MCP test and confirmed the docs'
historical/uncertainty boundary. No source edits, game or deployment operations
were performed by reviewers.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.86,
request 9d4fae6d5f2239986ae63d066cace744c92987ee09843858c07943090e8368de;
655 input and 44 output tokens, 0.288 seconds. Author accepted the bounded
source change on direct tests and both independent reviews. Installation,
deployment, native qualification and live testing remain deferred.
