# Local public catalogue search

Install optional dependencies in a project environment.

```powershell
python -m venv --system-site-packages .venv
./.venv/Scripts/python.exe -m pip install -r requirements-knowledge.txt
./.venv/Scripts/python.exe -m tools.sync_forever_embedding --output local-data/knowledge-model
./.venv/Scripts/python.exe -m tools.forever_knowledge build --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model
```

The existing catalogue sync command supplies the pinned catalogue.
Model sync is explicit. Build, search and MCP serving use local data.

```powershell
./.venv/Scripts/python.exe -m tools.forever_knowledge search --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model --query "Garb of Thero-shan" --section items --limit 5
./.venv/Scripts/python.exe -m tools.forever_knowledge serve --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model
```

The local ICARUS registration is named `azeroth-knowledge`. It allows
only `search_forever_knowledge`, sets a 60-second startup allowance and
a 30-second tool timeout, and supplies offline model/telemetry flags.
The saved entry was launched independently through stdio and returned the
expected catalogue IDs. A client must refresh its MCP connections to
discover a newly registered server.

For another MCP client, launch the second command from this repository's root,
using absolute paths for the interpreter and all three configured inputs.
Its tool is `search_forever_knowledge`. Allowed section filters are
`items`, `npcs`, `zones`, `factions`, `consumables`, `enchants`,
`random_suffixes`, `spell_effects` and `encounters`. Limit is 1–20.
Use returned section and record IDs with the main catalogue tools for
exact facts. Similarity scores describe retrieval.
Encounter identities are paths. Enchant IDs are effect IDs and can match
multiple source variants; their distinct record keys include a hash of
the complete source row. Spell effects without names use a generated
numeric label. No descriptions are inferred.
Set `include_records: true` with a limit of 1–5 to include the exact
pinned catalogue row and its canonical SHA-256 with each candidate.
The combined response is bounded to 256 KiB. The CLI equivalent is
`--include-records --limit 5`.
For `section: "items"`, supply `item_ids` to search only an explicit
candidate list, such as IDs obtained from the owned-gear review. Filtering
applies before both retrieval rankings. Lists may contain up to 10,000
distinct positive IDs. An empty list gives no results. The CLI accepts
repeated `--item-id` options. The index retains public reference records;
supplied candidate IDs do not establish ownership or equip legality.

The index contains public reference records. Its builder accepts the pinned
catalogue and verifies its checksum. Search makes no source refresh or game
call. Personal snapshot data stays in the existing owner-scoped store.

Every query retains full source/index/vector validation. Norm-tolerance checks
are combined after the unchanged per-vector dimension, finite-value and norm
calculations. A small alternating warm benchmark on this host measured 38.1%
lower median lookup latency with identical replies; model startup is excluded.
No query API, index format, model, persistent query cache or privacy boundary
changed. See local-knowledge-validation-benchmark-2026-11-10.json for the scoped
measurements and limitations.

Set include_references: true (CLI --include-references) with limit 1..5 to attach
the base row/hash plus direct NPC, zone and faction links. Each link identifies
its exact source_path into the base row and either returns the target's exact
public record/hash or not_in_pinned_catalogue. Unknown targets stay unknown;
no remote lookup, name matching or recursive enrichment occurs.

Supported links are top-level zoneId and npcId/zoneId/repFactionId inside source
objects. Other relationship types are outside this projection. Up to 32 edges
per hit are returned, with complete recognised-edge count and truncation flag;
several edges may reference one entity. The combined base/related response
retains the 256 KiB bound. References use catalogue IDs and do not verify native
world identity, current acquisition availability or a complete reference graph.
Private snapshots and the public index remain unchanged. Existing queries that
omit the option keep their previous response shape.

Reference results also link listed randomSuffixOptions to exact random_suffixes
rows. They are labelled catalogue_option_not_owned_variant, with
owned_variant_verified: false. Public possibilities never establish an owned
modifier, effective item stats, simulator resolution or equip eligibility.

Use reference_offset (CLI --reference-offset) with include_references to page
all recognised edges, 32 at a time. It is a uint32 starting at zero and applies
to every hit. Keep query/filter/base key/hash and pinned source unchanged, and
advance by 32 while has_more_references is true. Beyond-end pages are empty.
references_truncated can stay true on the last page because earlier edges are
omitted from that response; use has_more_references to continue. World links
come first, then suffix options in their original array order. The combined
256KiB bound, exact records/hashes and privacy boundary remain unchanged.
