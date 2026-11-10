# Encounter-specific saved gear leaders

Date: 10 November 2026. Status: source/offline only.

The aggregate top list can omit the best completed set for a particular
encounter. Add opt-in `--include-encounter-leaders` to
`tools.run_forever_saved_journal` and `tools.evaluate_forever_owned_gear`.
Retain one point-estimate leader for each supplied encounter (at most eight),
independently of the aggregate top limit and weighted/worst-encounter objective.
Use the existing role-normalised improvement: larger improvement is always
better, including damage taken, death fraction and time out of mana.

Use only candidates compared with the equipped baseline in every supplied
encounter. Apply the same across-encounter improvement floor before either
ranking. The baseline participates when it meets the floor. Resolve equal
encounter improvements by ascending simulator gear SHA-256; this is a
deterministic display choice, not a physical-copy preference or statistical claim.

An incomplete page previously returned only its aggregate top candidates.
When requested, retain up to eight additional distinct encounter leaders before
that truncation. The saved runner includes these bounded rows without adding
simulations. Keep all existing candidate, identity, budget and journal guards.
Join leader references through the existing validated saved-loadout reader;
at most eight distinct additional sets enter this separate bounded join.

The optional report states its existing ranking scope and completion status,
lists encounters without an eligible completed candidate, and keeps statistical
superiority false. A partial run yields partial leaders. Even full saved-result
coverage does not prove complete ownership, feasible-set coverage, model
acceptance, statistical optimality or BiS. Native eligibility and current item
locations remain unverified. No equip, game interaction, sync or public export
is added. Default output and aggregate rank order remain unchanged.
