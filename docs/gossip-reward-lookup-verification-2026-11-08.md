# Saved gossip quest reward lookup — 8 November 2026

`lookup_forever_catalogue` adds item filters `gossip_quest_list` (`available` or
`active`) and optional `gossip_quest_is_complete` Boolean. A list filter requires
an owner-local Forever `acquisition_snapshot_id`; a completion predicate requires
the list filter. Other catalogue sections are rejected for these filters.

Example:

```json
{"section":"items","gossip_quest_list":"active","gossip_quest_is_complete":true,"acquisition_snapshot_id":"<saved snapshot ID>"}
```

The query finds pinned catalogue quest reward associations whose source quest ID
was observed in the selected saved gossip list. Completion must be explicitly
known and match the requested Boolean. Unknown does not match false. If
`reward_quest_id` is also supplied, the exact ID, membership and completion
conditions must all hold on one source. A different quest for the same reward
cannot satisfy the other half of the predicate. Existing item filters, source
references, ordering and 20-record pagination remain available.

`gossip_quest_filter` records the selected list, whether it was observed, nullable
observed quest count, requested completion and false eligibility verification.
A missing list yields no observed-source matches with `list_observed: false` and
null count. An observed empty list yields `true` and zero. Neither absence of
matches proves no rewards exist. Old queries without the list filter omit this
new filter-description field.

Item acquisition quest sources independently annotate observed available/active
gossip rows and their nullable completion flags. Root acquisition progress reports
which dialogue lists were observed. Quest-log presence, readiness, failure and
objective data keep their existing meaning and are not filled from dialogue data.
The progress retains its source snapshot ID and capture time.

These are historical reference joins, not proof of current native quest rewards,
reward choice, NPC identity/continuity, eligibility, acceptance or turn-in. The
catalogue can be incomplete or stale. No quest, dialogue or simulator action,
new native API call, external lookup, install or deployment is added.

## Offline verification

- `cargo build --locked --offline` passed for the changed source.
- `PYTHONPATH=.;tests python -m unittest test_gossip_reward_lookup test_gossip_quests test_catalogue_recipe_learning`:
  three real MCP tests passed in 4.071 seconds. New coverage includes source
  correlation using item 1490's two quest sources, true/false/unknown completion,
  available/active lists, missing versus empty, separate quest-log annotations,
  preserved legacy reward lookup and rejected malformed/incompatible filters.
- Formatting and diff checks passed.

Independent Callers and Failure/security reviewers (`gpt-6-astra`, high effort)
found no concrete defect in one round. Both ran the new real MCP test. Callers
also ran existing gossip/protocol tests, matched 218 reward records against the
pinned catalogue across 11 pages, and passed 16 same-source oracle comparisons.
Failure/security passed 29 additional assertions/cases covering invalid filters,
foreign owner stores, Retail/Classic, malformed saved captures, traversal,
opposite completion flags on different sources and missing completion. Errors
did not expose supplied private character names or paths. Reviewers reused the
prebuilt binary; author compilation is separate current-source evidence.

Jev plan advisory: `jev-1.14.0`, request
`109ff8eb55ce723bad1d589066edd0c881602d7b2f539756851f95b87fce7df8`,
492 input / 47 output tokens, 0.271 seconds; proceed probability 0.90. Advisory
confidence does not verify catalogue/native reward equivalence or eligibility.

Post-verification advisory: `jev-1.14.0`, request
`cff136b886e1f4899fccb356d43b6455ea14be2da72f9ba0390be9f88c04cff4`,
563 input / 50 output tokens, 0.325 seconds; land-scoped probability 0.97.
Scoped tests and independent source/caller checks provide the actual evidence.
