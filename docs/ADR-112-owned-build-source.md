# ADR 112: owner-bound saved character sources

Date: 16 November 2026

The optimiser needs a real saved character without guessing that the newest alt
is in play. A configured account export identifies the character at capture time.
It does not establish current play after that capture.

`OwnedBuildSource` fixes an owner-local account index, snapshot directory,
exact Forever client identity and addon export. The default selection reads
the saved export through the existing Rust data-only parser. The account index
must point to a hash-checked snapshot with the same complete observation.
Equivalent RFC3339 capture instants and the schema's absent/empty `unavailable`
map compare equally. Every other exported field must match exactly.

An explicit name and realm take precedence over the export. Missing, ambiguous,
wrong-client, future or changed sources fail without another-character fallback.
The owner account key remains fixed outside tool arguments. The index separates
accounts, realms and client variants. Selection filters exact build and interface;
saved faction metadata remains attached to its character.
Missing storage stays unknown; this source does not combine old banks, mail,
shared banks or another character's items into available stock.

The new `decode-export <file>` CLI uses `import::import_file` and serialises the
validated snapshot to stdout. It does not ingest, upload, execute Lua or touch
the game. Python captures that bounded output internally and suppresses errors.
The source and its saved-data caller are read-only. Configured paths reject
reparse points; private data files reject hard links. The trusted executable
can retain Cargo's normal hard link. Tool arguments never select file paths.

Results identify `saved_active_export` or `explicit_saved_character`, capture age,
`live:false` and `active_character_verified:false`. Historical saved data is
valid offline input; it is not a fresh current-session assertion.

Validation uses real Rust decoding, the account index and hash-bound snapshots:

```powershell
./.venv/Scripts/python.exe -B -m unittest discover -s tests -p test_owned_build_source.py -v
```

The next adapter connects this source to owned gear/talent generation and native
joint windows. The native model's level-60 restriction remains in force.
