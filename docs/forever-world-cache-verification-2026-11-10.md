# Pinned Forever world cache evidence

Scope: ADR-105. The explicit public source sync and local Generation succeeded.
No installed addon or MCP profile changed. No game process, private capture or
private character store was accessed.

Source: Questie/QuestieDB commit `aca0da1c4a5ed0bc12997b9aaeef7b35af9ee26e`,
148 pinned files, 26,552,824 bytes. The full source archive exceeded the 64 MiB
probe cap; the failed temporary download was removed. The implemented sparse
sync succeeded and retained notices/provenance. The local Lua interpreter was
hash-pinned with all other staged inputs. No upstream payload is committed.

The offline build produced 20,278,256 bytes with SHA-256
`f535e174c7ce519442d91d9888bc81e5e5bc47dac436f72297c7664741cc15e6`.
The Generation header retained contract 4, 104,905 static corrections and 3
derived passes. Counts: 5,682 quests, 15,559 NPCs, 7,258 objects and 23,858 items
(52,357 total). These are exact local export counts, not world coverage claims.

Actual CLI search for `Hogger` in quests returned the exact record key
`questiedb:quest:176`. Exact get returned the provider's quest fields and row
hash; the finisher tuple retained NPC ID 240. This does not qualify native
availability or character-specific rules.

The first MCP test harness blocked `subprocess.Popen` before Windows' asyncio
import, which stopped startup. The corrected harness blocks `subprocess.run`,
the build/sync entry point, without replacing the Windows process base class.
The actual MCP test passed in 2.527 seconds with external socket connects
blocked. It checked both tools, four kinds against source lines and hashes,
counts, fixed pages, missing IDs, malformed bounds and unchanged cache bytes.

Final author command, with `PYTHONPATH=.;tests` and TEMP/TMP `C:/Temp/codex`:

```
.venv/Scripts/python.exe -B -m unittest test_forever_world
Ran 2 tests in 11.109s
OK
```

This also regenerated the exact cache under an ambient Lua-init sentinel,
verified no-op source sync without subprocess access, rejected changed source
bytes before interpreter launch and preserved an unrelated existing output.
`git diff --check` passed.

## Independent review

One round, two independent gpt-6-astra/high agents: callers/failure and
security/data. Neither found a concrete fault. Both read the Python/Lua modules,
pin and ADR-105. Both ran the two scoped tests through unittest discovery with
`-p test_forever_world.py`: callers 12.015 seconds and security 10.035 seconds,
both OK. The author run above includes the final added no-op sync assertion.

The callers reviewer compared all 52,357 exact lookups with their source rows.
Maximum reply size was 102,598 bytes. Its independent search oracle passed eight
queries at six offsets. Additional probes checked detached results, no-op source
verification, no-overwrite publication, 45-second generator timeout cleanup,
and sparse keys, zero, false, escaping and numeric/string key collision handling
in the Lua serializer. Non-finite values were rejected.

The security reviewer confirmed no file, process or socket operations during
get/search after cache load. Linked input and corrupt caches were rejected,
with unrelated bytes preserved. A pinned export under hostile Lua-init/path
settings matched the expected digest. An injected destination race preserved
the competing file. The restrictions were checked as defence in depth, not as
an OS sandbox. Both review lenses kept network sync outside their offline tests.

No review source writes, installation, private captures or game access. Jev did
not run because its tool declares external paid inference and no separate
spending approval exists. Author judgement accepted the direct evidence and
both clean reviews. Agent usage was not exposed by the tool. Native availability
and installed MCP discovery remain untested.
