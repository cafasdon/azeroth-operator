# ADR 014: Non-evicting numeric simulation checkpoints

Date: 9 November 2026. Status: source/offline implementation.

The optional cache in ADR 007 intentionally evicts after 128 entries and treats
corruption/storage failure as a miss. It cannot serve as a durable completion
record for a larger optimisation search. A separate `SimulationJournal` now
implements the fixed adapter's key/get/put interface without eviction. Both
fixed-run and gear-ranking CLIs accept an explicit absolute private root.

Keys include engine SHA, complete canonical request, paired/tank/healing modes,
cache protocol identity and journal/adapter source hashes. Updating those sources
produces new keys; old results are not overwritten. Engine verification still
runs on every adapter call, including hits. Existing model checks precede
checkpoint use. Data is bound to requested engine, iterations, seed and modes.

Only known numeric summaries and bounded per-iteration numeric samples are
stored, never the raid/APL/request, character names or raw engine output. A
canonical key+summary envelope binds plaintext records to the exact request key
on every platform. Windows additionally uses existing current-user DPAPI with
key entropy and native owner/DACL checks on root/database/sidecars. POSIX requires
current-user ownership and private modes; plaintext is not encrypted there.
This is integrity/privacy within those boundaries, not protection from malicious
code already running as that OS user or authenticated provenance of simulations.

SQLite has a dedicated application ID/version and exact schema. Foreign files,
unknown schemas, links and unsafe permissions reject. Values/SQL length and
checked deadlines are bounded. Transactions preserve earlier results on
failure. Identical retries are idempotent; conflicting results and corrupt data
stop rather than overwrite or silently recompute. No entry is evicted.

The default ciphertext payload budget is 64 MiB, configurable 1–512 MiB; the
entry cap is one million. Each plaintext envelope is at most 512 KiB. SQLite
metadata/journals add disk overhead, so these are logical rather than physical
file quotas. Exceeding a quota rejects the new write. Existing records remain
readable even if the selected budget is lower than their total.

CLI `--simulation-journal` excludes optional `--cache`, plan-only mode and staged
bulk adapters. Fixed comparisons, paired confirmation and tank/healing rankings
are supported. Staged telemetry retains the existing optional cache. The journal
alone does not schedule jobs, execute prepared sets, prove complete feasible
coverage, rank all owned gear or qualify live encounters. Those remain work.
