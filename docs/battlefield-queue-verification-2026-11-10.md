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
