# Larger owned candidate scans — offline evidence, 2026-11-09

Previous goal turn was progress: e20734d added group context. Initial worktree
was clean. Modifier resolution already distinguishes full simulator item specs;
the new gap found here was physical pool admission before equivalent-copy collapse.

Baseline real-MCP synthetic probe: 257 identical captured rings across both ring
slots hit the old 256-location GearSearchLimit. The error is wrapped by the MCP
SDK's nested task groups; the test was corrected to assert its exact leaf cause
rather than mistake that wrapper for another failure. Baseline rejection was
then confirmed before implementation. ADR-073 defines the new explicit budget.

Author command, PYTHONPATH=.;tests and TEMP/TMP=C:/Temp:

```
python -m unittest test_candidate_scan_limit
Ran 3 tests in 18.171s — OK, no skips
```

The actual local MCP and pinned catalogue are used with synthetic snapshots:

- Default 256 still rejects the complete 257-location pool; no escalation.
- Explicit budgets 257 and 300 produce the same pool scope. The two ring slots
  retain distinct physical copies and all 510 excess-copy exclusions, proving
  both complete pools were consumed.
- A second 257-location pool has a distinct suffix on its final location.
  Both suffixes survive, with 508 equivalent-copy exclusions and 16 product
  coordinates. This checks late-page modifier preservation rather than assuming
  base item identity implies equivalence.
- A 4,096-location valid-reader fixture completes under the existing 45-second
  test bound, preserving one representative and 4,095 copy exclusions. Synthetic
  slot counts are schema tests, not evidence of native bag capacity.
- Real window collection resumes from budget 257 to 300 with unchanged binding
  and a second saved window. Source/version fingerprint rules remain unchanged.
- Invalid boolean, string, float, null and out-of-range budgets reject before
  execution I/O. All three real module --help invocations expose the new option.

The implementation writer and independent Callers/Failure reviewers used
gpt-6-astra/high; actual agent token usage was not exposed. Callers independently
ran all three tests in 19.817s, clean. Failure ran them in 20.461s, clean, plus
24 budget/default/boundary probes, an exact 82-page scan at 4,096 locations,
four rejected pagination faults and thirteen invalid-budget no-I/O/privacy checks.
Over-budget counts reject after the first page, returning no candidate prefix.
No engine, Rust/addon rebuild, network, deployment or game access in this unit.

Jev plan advisory jev-1.14.0 request
1be86fb1403cc942952506d3b6bbf48d24f848395a23b2f3fedb94383250f11e:
proceed .81, revise .14, insufficient .05, confidence .72; 628 input / 47 output,
.290s. Advisory only. Existing complete-ownership, legality, model and full-BiS
qualification gaps remain; broad MCP coverage stays active.

Callers performed a final documentation-only pass without repeating tests:
ADR-073, README, changelog and client/audit claims match the implementation.
Final Jev advisory jev-1.14.0 request
e1732b0e141358172064cdcf37ee9bd34386a442639de867a79b5e62205afb26:
integrate .87, revise .07, insufficient .06, confidence .80; 641 input / 50
output tokens, .332s. Author accepts the bounded source change on direct tests
and independent evidence; no complete owned-BiS qualification is asserted.
