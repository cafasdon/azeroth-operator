# Item lookup cooldown integration — 9 November 2026

## Intended behaviour

Connect passive carried-item cooldown observations to the existing item lookup.
Join by exact bag/slot/item ID within the same validated snapshot. Only matching
bag locations that the existing item lookup counts receive `captured_cooldown`.
Its data remains historical; outer capture time applies. Equipment and bank
locations never borrow bag cooldowns, even with the same item ID or slot key.

When the section exists, `bag_cooldown_observation` reports matching bag location
counts before pagination: reported, unavailable, disabled and positive timers.
It explicitly leaves current usability unverified and game execution false.
Quantities, matches, ordering and paging remain unchanged. When absent, no new
fields are added. Multiple explicitly selected snapshots are queried separately
and never exchange cooldown metadata across characters or timestamps.

## Author evidence

- `cargo fmt` and `cargo build --locked --offline`: passed; build 7.38s.
- `PYTHONPATH=.;tests python -m unittest test_item_cooldown_lookup`: one real
  local Rust stdio MCP test, .210s, OK. Same base item in equipment, three bag
  slots and character bank; distinct per-slot timer/status; unrelated item
  excluded from counters; seven offsets including beyond the end; totals and
  page slices unchanged; missing-item zero counters; exact legacy output
  comparison; separate newer alt and legacy capture; unchanged snapshot bytes.
- `python -m unittest test_loot_window test_mail_attachments`: two existing
  actual MCP inventory-boundary tests, 1.097s, OK. No pending loot/mail credit
  or existing lookup behaviour changed.

All captures are synthetic in an owner-local temporary store. No game process,
external lookup, real character, installation or deployment. Addon capture and
package are unchanged from the prior uninstalled 0.46.0-dev qualification.

## Plan advisory

Jev 1.14.0 request
`c347934dba27a060e6204e64629ae6794ed967fc74ddd0a99023d99f7463e1ed`:
proceed .86 / revise .10 / insufficient .04; 606 input, 47 output, .311s.
Advisory only; code and direct tests are the behaviour evidence.

## Independent review and disposition

One clean round: gpt-6-astra high Callers and Failure/privacy. Both ran the
actual lookup test (.217s/.297s), OK. Callers traced all find_item consumers and
probed cooldown-only changes through item comparison/spending budgets: quantities
unchanged. Failure probed same-character captures with different timers,
malformed stored source linkage (rejected), and maximum offset (empty page,
pre-pagination counters retained). No current-diff fault found. Native/deployment
lens omitted because no such operations occur. Agent token usage unavailable.

Non-blocking existing issue recorded for focused follow-up: Python ingestion
accepts a bag row missing count; Rust rejects it with or without cooldown data.
Author reproduced Python acceptance and inspected unchanged schema.rs's positive
stack-count requirement at baseline 9a977df. Normal addon capture supplies a
count and Rust fails closed. This lookup unit neither creates nor repairs that
pre-existing admission mismatch. Next action: align Python item validation with
the authoritative Rust contract and add malformed-source parity regressions.

Final Jev 1.14.0 advisory request
`f3beb9b3962efead61ecabfcf44e7259700eb099ceb7d3fd7d405342de55945b`:
land .96 / fix .04 / insufficient .00; existing-gap record-separately 1.00;
716 input, 96 output, .303s. Author disposition: land the bounded join on direct
evidence and two clean reviews, keep the existing validation gap and broad goal
open. No native usability or full BiS claim.

Subsequent status: the recorded stack-count discrepancy is repaired by the
focused Python count guard; see
[stack-count parity evidence](item-stack-count-parity-verification-2026-11-09.md).
This does not claim full schema parity beyond the tested count contract.
