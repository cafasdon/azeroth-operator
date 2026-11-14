# Incremental public reference cache

`tools.reference_sync` stores the two existing pinned public providers:
Questie/QuestieDB world sources and the WoWSims Forever catalogue. It does not
scrape websites, extract client archives or accept arbitrary hosts. Source
locks name an immutable commit and SHA-256 for every file.

Unchanged hashes reuse local objects across revisions. New or changed files
download once, with bounded HTTPS reads and checksum validation. A changed
upstream JSON file still needs that complete file; neither provider supplies
a general row-delta API. This is file-level incremental sync, not a claim of
byte-level patches or automatic detection of the correct game patch.

## Retention and limits

Each cache has one active manifest. All needed new objects must validate
before atomic promotion. After promotion, objects outside the new manifest
are removed. Inactive objects from interrupted updates are reclaimed safely
before staging, while active and requested objects remain protected.
No old patch history accumulates in this cache.

The persistent ownership marker fixes its byte budget and free-disk reserve.
Existing limits cannot grow implicitly. The total includes objects, metadata,
staging and the stable OS-lock file. Each streamed write checks remaining
space before writing. An update that cannot fit beside the active data stops
without deleting that active version. Unknown files, links, corrupted objects
and changed ownership markers fail closed. Cleanup addresses owned regular
cache files only, never private snapshots or unrelated directories.

The installed pilot caches use these limits:

| Cache | Active bytes | Hard limit | Free-disk reserve |
| --- | ---: | ---: | ---: |
| World source objects | 26,117,166 | 64 MiB | 64 MiB |
| Catalogue | 2,498,591 | 16 MiB | 64 MiB |

These are limits for these managed roots, not the whole project or machine.
Existing compiled world/knowledge caches and original owner snapshots are
separate. The source cache does not silently delete or rewrite them.

## Use

From the project root, seed the cache from already verified local inputs:

```powershell
./.venv/Scripts/python.exe -B -m tools.reference_sync sync --cache "ABSOLUTE_WORLD_CACHE" --lock clients/forever/world.lock.json --seed "ABSOLUTE_PINNED_WORLD_SOURCE"
./.venv/Scripts/python.exe -B -m tools.reference_sync sync --cache "ABSOLUTE_CATALOGUE_CACHE" --lock clients/forever/catalogue.lock.json --seed "ABSOLUTE_CATALOGUE_JSON"
```

After a reviewed lock update, use the same cache and omit `--seed`. Only missing
hashes download. `status`, `query --name SOURCE_PATH` and `read-file --name
SOURCE_PATH` operate offline. Initial `--max-bytes` and `--reserve-bytes` options
set a new cache's limits. A fresh cache otherwise defaults to 256 MiB and a
64 MiB reserve. The actual pilot roots above explicitly use smaller budgets.

Existing MCP readers continue to use their validated compiled caches. A source
sync does not automatically promote a new build or overwrite an unknown file.
Generation and reader promotion remain explicit, checksum-checked operations.

## Evidence

Both real caches were seeded from local data with a downloader that raised
on any network attempt. World seeding stored 145 unique objects for 148 files;
the catalogue stored one object. A second sync without a seed downloaded zero
bytes and reused all 148 world files plus the catalogue file.

Twenty-six focused tests passed. Simulated revisions fetched only the changed
file and removed superseded data. Real child-process tests covered crash,
restart and lock contention. Quota, free-space, hash, promotion, path and
ownership failures preserved active data.

Independent reviews found four normal caller defects: an orphaned lock after
crash, duplicate staging on a full no-op, unreclaimed interrupted objects and
missing nested parent creation. All were fixed and scoped rechecks passed,
within three bounded rounds. No open medium/high or low finding remained.
No pipeline or live network fetch ran for this acceptance check.
