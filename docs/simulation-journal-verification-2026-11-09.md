# Numeric simulation checkpoints — 9 November 2026

Scope: source, real SQLite/current-user Windows DPAPI and synthetic engine
responses through existing adapter/caller paths. No real simulation engine,
native game, install, deployment, external sync or CI. The broad objective and
full owned BiS remain incomplete.

Both `python -m tools.run_forever_sim` and `python -m tools.rank_forever_slot`
accept `--simulation-journal ROOT`. The root must be absolute and private under
the existing native owner/ACL policy. `--simulation-journal-mib` configures the
ciphertext budget, defaults to 64 and rejects outside 1–512. A budget without a
journal, optional cache together with journal, plan-only mode and staged bulk
combination reject. Fixed-run metric modes and confirmation keep their guards.

The new store does not evict: 130 independent numeric results survived a new
instance and all were readable. Its key includes the full request, engine,
modes and software source identity without storing requests. The native adapter
and CLI were exercised with a synthetic engine subprocess stub: a second store
instance reused the result without another launch, while changed engine bytes
still rejected before reuse. These are checkpoint/caller tests, not simulator
performance or model-accuracy evidence.

Tests cover all eight paired/tank/healing combinations, exact numeric field
whitelists, sample counts, seed/iteration/engine binding, invalid metrics,
Windows DPAPI key substitution, private text absence, quota and conflict rollback,
idempotency and foreign files. An oversized stored BLOB is rejected by SQLite
before Python row decoding. Default optional-cache regressions remain separate.

Author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_simulation_journal -v
4 tests passed in 1.809s (initial library)
python -m unittest test_simulation_journal test_simulation_cache -q
14 tests passed in 3.383s
python -m unittest test_simulation_journal test_simulation_cache test_simulator_level -q
19 tests passed in 5.280s; latest CLI budget version 5.191s
```

The module and direct-script simulator help paths and module ranker help passed.
The direct ranker script import limitation predates this change; use its module
entry point. No unrelated import-path change was made.

## Independent review and correction

Round one used `gpt-6-astra`, high effort, Callers and Failure/security.
Per-review token usage was not exposed. Callers ran the five-test discovery
module (3.899 seconds) and independent synthetic probes through actual `run()`
and ranker `main()`: all eight metric-mode combinations hit after one miss;
all six ranking modes reused results; seed confirmation needed only two new
synthetic launches. Both CLI budget/cache guards, plan-only and staged bulk
rejections passed. Default and existing engine/model checks remained intact.

Failure/security reproduced one P2: POSIX identity protection left a plain
summary unbound to its row key. Transplanting body/digest/size under another
request key with the same engine/seed/iterations/modes returned the wrong result.
This was reproduced by the author using the exact plaintext identity branch on
Windows. The new regression failed before the fix (`ValueError not raised`).
The surgical fix adds the key+summary envelope described in ADR 014; no request
or identity payload was added. The same regression and full scoped module then
passed: six tests, 3.550 seconds. Native POSIX permissions are not qualified by
this Windows identity-branch test.

Round-one Failure/security also verified eight concurrent workers: 48 new/
identical writes and readbacks produced exactly 24 rows. A forced exception
after a synthetic DELETE rolled back with byte-identical database contents and
the prior value readable. No real engine was used.

Jev finding triage: `jev-1.14.0`, request
`c6958081a701c3355b3defbec800533417a003d0db5f119f20a6b3f0f95dc0c0`,
614 input / 48 output tokens, 0.271 s; fix-first 1.0. Advisory only.

Round two used Failure/security only: the fix touched no new caller or file,
so Callers was omitted. Command:

```text
python -m unittest discover -s tests -p test_simulation_journal.py -v
Ran 6 tests in 3.788s — OK
```

Independent corrected-branch probes returned:

```text
original_POSIX_substitution=ValueError
failed_read_database_unchanged=True
original_key_still_readable=True
windows_dpapi valid_roundtrip=True rejected=wrong_key,missing_key,extra_field,old_plain_summary,duplicate_key,noncanonical,oversized_plaintext
posix_identity valid_roundtrip=True rejected=wrong_key,missing_key,extra_field,old_plain_summary,duplicate_key,noncanonical,oversized_plaintext
```

Malformed envelope probes used matching body hashes and declared lengths, so
envelope checks themselves caused rejection. The round was clean. All owned
synthetic temporary data was cleaned and SQLite connections closed; protected
old directories were untouched. No real simulation, game or network access
occurred. Cross-user logon and native POSIX permission behaviour were not tested.

Post-fix scoped advisory: `jev-1.14.0`, request
`89eeefcfe200c613c77e0528b8a44c10bca148114f109d1682cde31f4f949ce6`,
927 input / 52 output tokens, 0.327 s; land-scoped 0.95, fix-first 0.05,
insufficient evidence 0.0, confidence 0.92. Advisory only, not proof or permission.
