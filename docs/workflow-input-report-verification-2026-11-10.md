# Workflow input report: offline evidence

Scope: ADR-085, source and synthetic offline MCP caller only. No installation,
deployment or live game test.

Commands and results:

- `cargo build --offline`: passed.
- `cargo test --lib workflow_coverage --offline`: 3 passed, 88 filtered.
- `PYTHONPATH=.;tests python -m unittest test_workflow_coverage`: 1 passed.

The MCP SDK test starts the real stdio binary with synthetic owner and foreign
stores. It checks all 24 profiles, observed empty versus absent/unavailable,
counts and missing lists, route metadata, omitted/false compatibility, explicit
snapshot isolation despite a newer different character, unchanged active-unknown
behaviour, invalid boolean values, Retail refusal, foreign-store refusal and
unchanged snapshot bytes. It checks that private reason text is absent from
the new report. Existing authorised summary fields retain their prior contract.

Author tests found two test assumptions: the owned-gear route is conditional
on a catalogue, and resolve_character needs family. The report now explicitly
marks the catalogue dependency; the caller test uses the actual parameter.

Plan advisory: Jev 1.14.0 chose workflow_report, confidence 1.0; hash
f00212794ee8090729e49ff06a88cbb4f88f66c5702fc06d6c75ca03faef1966,
641 input / 47 output tokens, 0.273 seconds. Advisory is not native proof.

Independent round 1: two gpt-6-astra/high reviewers (failure and callers/privacy)
reproduced the same test isolation fault. An inherited optional catalogue setting
made the test's absent-route assertion fail. Root reproduced it with
`AZEROTH_FOREVER_CATALOGUE=C:/Temp/azeroth-review-no-catalogue.json`.
The test now removes only that setting from its synthetic child environment.
The same command passed afterward: 1 test, 0.249 seconds.

Jev triage: fix, confidence 1.0, resolved jev-1.14.0, hash
10f765c96d7527a81c9c4b4ef634a97cce6f6007b656b181312c03827c4db57e,
473 input / 46 output tokens, 0.317 seconds. Production routing was unchanged.

Reviewers also exercised all-unavailable profiles, invalid section forms,
traversal/missing IDs, Classic refusal, strict Boolean errors without value
echoes, tools/list schema and saved-reset option composition. These probes passed.
No native behaviour was tested.

Independent round 2: fresh gpt-6-astra/high failure and callers/privacy reviewers
found no defect. Failure tested inherited catalogue values empty, relative and
missing absolute: all three test runs passed (0.469/0.279/0.517 seconds).
SDK inspection confirmed its default environment does not restore the removed
setting. Callers tested actual production routing with catalogue configuration
present and absent: route exposure differed as intended, reports were identical,
private values stayed excluded and synthetic store bytes stayed unchanged.
The inherited-catalogue regression passed independently in 0.244 seconds.
No live-drift lens applies: no installation or deployment occurred.

Final advisory: Jev 1.14.0 chose ready for source/offline only, confidence 0.98;
hash b99dcc52ea1a1b1bff3d70e846e63742450a396a3805efb2905ff4487d781af9,
574 input / 44 output tokens, 0.309 seconds. Root accepts the bounded source
result based on direct checks and independent review, not advisory probability.
Native acceptance, active identity and full game coverage remain unproven.
