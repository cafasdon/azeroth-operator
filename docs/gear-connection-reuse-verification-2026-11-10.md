# Gear connection reuse: offline evidence

Scope: ADR-090, local stdio reader, synthetic character, existing local catalogue.
No simulator, real character, network, installation or deployment.

Host: Windows/Python 3.14.7, AMD64. Reader SHA-256:
12deffa9fa6151dfc804b44484458faae9746afb68f36338809f96a5d9d4a8d1.
Catalogue SHA-256:
5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e.

Before editing, actual old collection ran three isolated nine-window trials:
3.5864 / 3.3560 / 3.3391 seconds, median 3.3560, nine MCP starts each.
Initial reuse trials gave median 1.8534, one start. A later run concurrent with
tests gave median 2.2661 and is not the controlled performance comparison.

Reproducible current-source command with PYTHONPATH=.;tests:
`python -B tests/benchmark_gear_connections.py`.
It alternates three fresh/reused trials over two synthetic rings, two slots,
nine coordinates/windows, same reader/catalogue and candidate-cache behaviour.
Fresh mode models the former connection lifetime: an outer placeholder supplies
no data and each window opens a real MCP connection. Reused mode runs production
collection unchanged. Both assert nine inserted windows and complete reported
coordinate coverage. All trials were sequential, outside other agent/test work.

| Trial | Fresh seconds / starts | Reused seconds / starts |
| --- | --- | --- |
| 1 | 4.1243 / 9 | 2.0865 / 1 |
| 2 | 3.6750 / 9 | 2.1763 / 1 |
| 3 | 3.6109 / 9 | 2.0754 / 1 |
| Median | 3.6750 / 9 | 2.0865 / 1 |

Fresh/reused median ratio: 1.7613, about 43% less elapsed time for this workload.
Three trials do not establish a universal speedup; the avoided eight reader
starts are directly counted. The benchmark is deterministic planning, not a
stochastic simulator evaluation.

Direct tests with PYTHONPATH=.;tests:

- New one-real-process/parity/lifecycle test plus existing resume/identity test:
  2 passed, 19.794 seconds. Actual SDK process handles were terminal after normal
  completion, injected second-window failure, resume and stalled initialization.
  Completed journal started no extra process. Saved draft/search/outcome/coverage
  fields matched separate standalone MCP calls exactly; input bytes stayed fixed.
- Existing source/template binding and collection timeout tests retained plain
  failure semantics and journal evidence.

Author corrections: the first shared context forwarded timeout failures through
SDK task groups; the old regression reproduced exception-group wrapping.
Explicit finally/aclose restored the existing error contract. A parity fixture
incorrectly disabled Hunter dual wield; matching the actual template context
restored exact scope hashes. No identity, timeout or source guard was weakened.

Additional direct checks: the existing above-1,024-coordinate collection case,
pre-first-journal timeout and committed-prefix handoff to simulation all passed
(3 tests, 18.031 seconds). These preserve large-window coordinate accounting and
the one-command evaluator's shared stage budget, not native simulator accuracy.

Independent round 1 (requested gpt-6-astra/high, failure and callers): both found
one P2 deadline regression. Startup and collection each received a fresh 45-second
allowance, permitting almost 90 seconds for the first window. Root confirmed the
old contract and fixed this with one window deadline, shared across startup and
collection. A real-reader regression advances the controller clock 30 seconds
during initialization and observes only about 15 seconds left for collection.
The total run deadline is also unchanged. Main model triaged this reproduced
contract fault; no external paid Jev call ran under the spending stop.

Other round-1 probes passed. Independent failure cases cancelled the owner,
crashed a background SDK task, changed template and changed snapshot during the
second window. Every case closed all children, retained one committed window and
resumed with two inserts. Handshake failure retained a zero-byte journal and
closed its process. ValueError/CancelledError remain plain for controller errors;
genuine background failures remain ExceptionGroup. Caller review ran the SDK
lifecycle and two evaluator budget tests (3 passed, 7.786 seconds); both actual
module help paths passed. Benchmark methodology/counters and reader/catalogue
hashes were independently checked; no timed run overlapped review work.

After the window-cap fix, lifecycle plus two evaluator budget checks passed
(3 tests, 9.030 seconds). The controlled benchmark was repeated sequentially
before any second-round agent tests:

| Final-code trial | Fresh seconds / starts | Reused seconds / starts |
| --- | --- | --- |
| 1 | 4.6717 / 9 | 2.1689 / 1 |
| 2 | 4.1528 / 9 | 2.3101 / 1 |
| 3 | 4.4123 / 9 | 2.6997 / 1 |
| Median | 4.4123 / 9 | 2.3101 / 1 |

Final median ratio: 1.9100. Timing varies; all reported trials remain recorded,
and only this bounded synthetic workload supports the speed comparison.

Independent round 2, fresh requested gpt-6-astra/high:

- Failure: no concrete defect. Lifecycle/skew-clock test passed (5.169 seconds).
  Isolated inherited-deadline probes confirmed a two-second run with 0.5-second
  startup leaves about 1.495 seconds for collection; exhausted startup calls no
  collection; already-expired input raises GearDeadlineExceeded. All context
  closes occurred in the owning task.
- Callers: no findings. Lifecycle plus pre-first-journal deadline test passed
  (2 tests, 5.235 seconds); committed-prefix evaluator recovery passed
  (1 test, 3.761 seconds). An in-memory mutation restored the renewed budget;
  the new regression rejected it with `45 not less than 16`. Source files stayed
  unchanged. No timed benchmark overlapped these reviews.

Root final own-model judgement: ready for the bounded source/offline optimisation.
No external paid review, real character access, build, installation, deployment
or native game test occurred. Live-drift lens was omitted because no live state
changed. Full ownership, legal combination coverage and native/model acceptance
remain unproven; the overall goal stays active.
