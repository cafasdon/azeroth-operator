# Explicit owner store: offline evidence

Scope: ADR-089. Source profile generation and pure upgrade preflight only.
No Docker, systemd, deployment, live credentials or private snapshot store access.

Intended result: no personal store default in generic rendering; one explicit
literal read-only mount; the existing owner-pinned upgrade still rejects another
store. Explicit configuration is not filesystem/OS ownership proof.

Direct commands with PYTHONPATH=.;tests:

- `python -B -m unittest test_gateway_profile test_gateway_upgrade`: 6 passed, 0.273 seconds.
- `python -B -m unittest test_recipe_search`: 1 passed, 0.369 seconds. Existing local MCP and synthetic recipe fixture only.
- `git diff --check`: passed.

Tests execute the real renderer module with an explicit synthetic Linux path,
with no store, and with mount-injection syntax. They assert stdout/error codes,
redacted supplied values, fixed read-only mount/environment, no environment/user
fallback, two distinct synthetic users, independently mutable results, invalid
image syntax and invalid path syntax. Pure check_current tests retain the exact
personal deployment pin and refuse a different synthetic user's store.

Author plan judgement: main model, explicit configuration plus fail-closed syntax
and unchanged deployment owner guards. No Jev call ran: the new spending stop
does not authorise external paid review calls. Independent reviews and final
own-model judgement below.

Independent round 1, requested gpt-6-astra/high:

- Callers: no actionable defect. All repository calls supply store except the
  deliberate missing-argument test. Both upgrade renderer calls use the local
  owner pin. Seven scoped tests passed in 0.642 seconds, including actual module
  invocation and synthetic real-MCP recipe/profile integration. Current guidance
  includes --store; dated evidence remains historical. Diff check passed.
- Failure/security: no material defect. Six profile/upgrade tests passed.
  Independent probes covered 256 character cases and 12 injection payloads,
  nested environment/tool/volume mutation isolation, the fixed upgrade store
  against three different paths, and five malformed profile values. Invalid CLI
  calls returned 2 with empty stdout, no traceback and no supplied-path echo.

Root's final own-model judgement: ready for the bounded source/offline change.
Syntax and explicit configuration do not prove OS ownership, symlink targets,
permissions or Docker acceptance. No live-drift lens applies because no installed
state changed. No external paid review call, deployment or private store read
occurred. Full game coverage and native acceptance remain unproven.
