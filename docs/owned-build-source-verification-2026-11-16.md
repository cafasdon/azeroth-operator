# Owned build source verification, 16 November 2026

The first source slice binds owner-local saved data without a live game test.
The actual configured Forever addon export selected its matching account-indexed,
hash-checked capture. An explicit selector selected the same saved character.
No saved payload, account identifier, character name or realm enters this record.

`cargo build --offline --bin azeroth-operator` passed. The new `decode-export`
route runs the existing native data-only parser without ingestion or upload.
Thirteen direct tests cover default versus newer alt, explicit override despite
missing/invalid export, account/realm/client boundaries, stale same-character
export, payload mismatch, missing/tampered snapshots, future capture, missing
storage, unchanged files and equivalent schema serialisations:

```powershell
./.venv/Scripts/python.exe -B -m unittest discover -s tests -p test_owned_build_source.py -q
```

Two independent Astra/high lenses covered callers/failures and security/data.
One medium finding caused valid default selection to fail after native JSON
normalisation. Timestamp instants and absent/empty `unavailable` now compare
equally; all other fields stay exact. The scoped security recheck passed:

```text
canonical default saved_active_export False False
offset default saved_active_export False False
empty unavailable default saved_active_export False False
canonical bytes unchanged True
offset bytes unchanged True
empty unavailable bytes unchanged True
hardlink refused
executable Lua 1 0 Error: Expected quoted data
tampered snapshot Snapshot content hash mismatch
```

The caller lens also exercised the actual stdio source session and
`inspect_snapshot`, with `live:false`, and refused foreign-account, missing-realm,
wrong-interface, changed-source and payload inputs. Its one wording finding
about account index keys was corrected directly. No medium/high findings remain.

Typed advisory evidence (`jev-1.14.0`; no private payloads submitted):

| Stage | Request SHA-256 | Input/output tokens | Seconds | Disposition |
| --- | --- | --- | --- | --- |
| Plan | `7c2a905eaeef994d5d64e3bda95c28fe69fc4510d5c4639c8d9c19dd8cd9064d` | 731/47 | 0.330 | proceed |
| Timestamp finding | `b3105ad650c3c61c679acedbf1b427a3f68e466dfcee0316a3b6084f50a48540` | 490/48 | 0.289 | fix medium |
| Empty-map finding | `b3b2d72c2e61a02066599b26558d0463ad8aa5bf79ca3887aedd8f43b3ef1cde` | 456/48 | 0.327 | fix medium |
| Source release | `8ac6994461759f574b8eff5d66308340bb1e8b01729d20f1d04af646d2d8a3b3` | 458/45 | 0.327 | ship |

Agent model requests were `gpt-6-astra/high`; resolved model and usage counters
were not exposed. Review used one full round and one scoped recheck.
Live identity, current storage and live model qualification remain unverified.
