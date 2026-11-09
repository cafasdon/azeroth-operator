# Item stack-count ingestion parity — 9 November 2026

## Problem and repair

At baseline 902039a, Python SnapshotStore ingestion accepted a bag item without
count. Rust's existing `schema::items` rejected it. This allowed Python to save
a capture that the MCP could not subsequently read.

Apply the same count contract in the existing Python storage-item loop:
equipment permits absent or null count only; bags, character bank and account
bank require strict integers from 1 through 4294967295. Booleans and integral
floats are not integers under this contract. No coercion, default count or
backfill. Rejection occurs before ingestion writes, and reads also validate.
Empty storage/equipment observations remain supported.

This is a count-only repair. Other shape/field validation and the Rust reader
are unchanged. It does not prove full cross-reader schema equivalence, complete
inventory, native behaviour or any new game operation. No existing owner data
is rewritten or deleted, and no addon/package/deployment changes occur.

## Author evidence

- Red: `python -m unittest test_item_stack_count_parity.ItemStackCountParityTests.test_missing_count_rejected_before_ingestion_writes`:
  one failure before the repair, `AssertionError: ValueError not raised`.
- Green: with `PYTHONPATH=.;tests`,
  `python -m unittest test_item_stack_count_parity test_bag_item_cooldowns test_item_cooldown_lookup`:
  four tests, 2.165 seconds, OK.
- `python -m unittest test_snapshots`: six tests, .036 seconds, OK.

The new actual local Rust stdio MCP test checks 120 cases: 15 count inputs
across equipment/bags for Retail, Classic and Forever, plus both Forever banks.
Inputs include missing/null, Boolean, zero/negative, positive, maximum/overflow,
integral/fractional float, string, array and map. Every Python admission agrees
with the unchanged Rust reader. Invalid Python ingestion leaves previously
saved file names and bytes unchanged; failed MCP errors omit supplied values.
All fixtures are synthetic temporary owner stores; no game, private character,
external data, installation or deployment.

## Independent review

One clean round with two gpt-6-astra high lenses: Callers and Failure/privacy.
Callers ran parity plus snapshot tests: eight tests, 1.331s, OK; bag cooldown
capture/lookup regressions: two tests, .544s, OK. Failure ran the two parity
tests against the actual MCP: 1.183s, OK. Both traced validation before
directory creation/writes and existing-read rejection; no concrete fault found.
Native/deployment lens omitted because these are explicitly deferred and no
such operation occurs. Agent token usage unavailable.

## Advisory plan

Jev 1.14.0 request
`705c8da302b8fbc058321874bd2527e441f67251721220169fbfba4055b196db`:
proceed .82 / revise .14 / insufficient .04; 526 input, 47 output, .273s.
Advisory only; the original red input and actual MCP parity cases are evidence.

Final Jev 1.14.0 advisory request
`30ad005d8780f15dec92671f36c07add5e98cac5b5515f47f0f09acac02e7d96`:
land .64 / fix .13 / insufficient .23; 607 input, 47 output, .285s.
Author disposition: land the six-line count repair on the original red test,
120 real MCP admission cases and two clean reviews. The advisory is recorded
unchanged; it does not establish wider schema parity or goal completion.
