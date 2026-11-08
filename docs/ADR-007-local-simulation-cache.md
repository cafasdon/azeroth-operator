# ADR 007: Optional local simulation result cache

Date: 2026-11-08
Status: accepted

## Context

Repeated fixed simulations run the same deterministic seeded request. Existing
deduplication only avoids repeats within one comparison. Saved snapshots and
simulator configuration can contain personal identifiers, so caching whole
requests or raw engine output is unnecessary.

## Decision

Expose `--cache` on both local simulation tools. Cache only the validated numeric
summary and required per-iteration numeric samples, never the request or raw
engine response. Hash the full canonical request, engine SHA-256, metric modes
and cache version into the key. Verify the engine on every request, including
hits. Keep paired samples in the cache unchanged; each read gets a fresh object.

Store entries in the OS user's local application-data directory. Windows uses
current-user DPAPI with the request key as additional entropy; it never selects
machine-wide protection. Native owner and DACL checks reject roots, existing
databases and journals granting other users access; they allow the current
user, SYSTEM, Administrators and owner rights. This also prevents another
user from injecting their own machine-scoped DPAPI payload into an insecure
store. See [Microsoft's DPAPI contract](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata).
The opaque request hash remains visible. This does not protect against code
already running as the same user or compromise of that user's credentials.
POSIX requires a current-user-owned private root and mode-0600 database.

SQLite transactions preserve writes under interruption. Keep at most 128 entries,
each at most 512 KiB before encryption; SQLite and temporary journals add overhead.
Check stored-body digests, decrypt with the matching key and bound reads.
Missing, unreadable, un-decryptable or byte-corrupt entries mean recomputation.
An unavailable cache location disables persistence without blocking simulation.

Bulk results remain uncached. Fixed finalist runs, audits and seed confirmation
can reuse fixed-run entries. Selection/search and snapshot collection still run,
so a cache hit does not imply an unchanged inventory or skip current validation.

## Evidence and limitations

Scoped tests cover real SQLite and Windows DPAPI, mode/input separation,
cross-instance reuse, key substitution rejection, corrupt/unavailable storage,
engine verification, paired samples and retention bounds. Cross-user logon was
not exercised; user binding follows the native DPAPI contract. POSIX permission
checks have not been exercised on this Windows host.

[Three public-fixture CLI cold/warm pairs](benchmarks/forever-cache-2026-11-08.json)
returned identical numeric output at distinct seeds. Warm elapsed time includes
process startup, imports, engine hashing, SQLite and decryption. This is an
offline reuse measurement, not native encounter validation.

Independent review found and prompted fixes for absent/relative LOCALAPPDATA
and pre-existing permissive Windows folders. Both reviews used gpt-6.1-sol at
high effort. Jev 1.14.0 advisory triage supported both fixes:

- Location fallback: request `fe3004c38cccc475a9a1c9f77eeedb8e4a5bb8a6aef4a693defce71c205684dc`,
  432 input / 45 output tokens, 0.343 s.
- Windows protection: request `80e363b5ef70a4ee2a3bd40f158daa1797e2d8e8cee7695f6ed41da9972aba14`,
  472 input / 48 output tokens, 0.305 s.
- Machine-scoped payload injection in writable folders: request
  `b10a4d39b8fbf79fad4e8a9b49a58acd321c317741e20ad31cf53d3a997aebeb`,
  459 input / 48 output tokens, 0.295 s; fixed by native ACL rejection.

Typed advice is not test evidence or a cross-user security test.

The third independent security and caller passes found no remaining scoped
defects. They rechecked the native ACL guard against permissive roots, databases
and SQLite journal files, machine-scoped ciphertext injection into an insecure
root, and native API failure. All used temporary files. The collected scoped
suite passed 12 tests on Windows. No live game tests were run.
