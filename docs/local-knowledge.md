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
That saved registration predates exact lookup and does not yet enable
`get_forever_knowledge_record`; enabling it in the installed client remains
deferred with deployment. The saved entry was launched independently through stdio and returned the
expected catalogue IDs. A client must refresh its MCP connections to
discover a newly registered server.

For another MCP client, launch the second command from this repository's root,
using absolute paths for the interpreter and all three configured inputs.
Its search tool is `search_forever_knowledge`. Allowed section filters are
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

When obtaining candidates with the main MCP's `lookup_forever_catalogue`,
`observed_ownership` checks saved base item IDs. Its
`ownership_filter_evidence` reports that scope and storage coverage; it does
not verify the current inventory or a particular owned roll. Combining it
with `item_random_suffix_id` finds base items listing that public suffix
possibility. `random_suffix_filter_evidence` explicitly labels the relationship
`catalogue_option_not_owned_variant`. Use the owned-gear review and exact
modifier resolution before treating a candidate as a usable owned variant.

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
objects, listed randomSuffixOptions and explicit effectIds. Other relationship
types are outside this projection. Up to 32 edges
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
come first, then suffix options, then effect IDs in their original array order. The combined
256KiB bound, exact records/hashes and privacy boundary remain unchanged.

Explicit effectIds link to spell_effects by their effect record ID, independently
of spellId. These expose exact pinned simulator effects for consumables, including
healing and resource effects where provided. Missing records stay unresolved;
an effect reference does not establish native behaviour, current usability or
a simulation result. The existing index needs no rebuild for these joins.

To find alternatives with the same explicit model effect, use the main MCP's
`lookup_forever_catalogue` with `section: "consumables"` and
`consumable_effect_id` set to the effect record ID. For example, `679187`
returns the consumables listing Minor Healing Potion's effect; spell ID `439`
is a different namespace. The filter composes with item IDs, key, group and
saved base-item presence, with normal 20-record pagination. It cannot be used
with restock targets or other sections. Its evidence does not establish equal
performance, native behaviour or current usability.

## Exact record lookup and reference traversal

The source companion also exposes `get_forever_knowledge_record`. Supply an
exact `record_key` returned by search or a resolved reference. It returns
`found: true` and one `record` containing the exact pinned catalogue row/hash,
section, identity, name and document hash. No similarity score or name-based
fallback is used. Unknown or noncanonical keys return `found: false` and
`record: null`; an enchant effect ID alone cannot select a hashed variant.

Use `include_references` and `reference_offset` with the same 32-edge pages,
uint32 offsets and 256 KiB response limit as search. Each call validates the
complete existing index, including unknown-key lookups, but does not encode a
query. This is retrieval of public source records; ownership, legality,
current availability and native behaviour remain unverified. No new index,
sync or model download is required. Service startup still loads its local model.

```powershell
./.venv/Scripts/python.exe -m tools.forever_knowledge get --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model --record-key consumables:118 --include-references
```

For example, follow the resulting `spell_effects:679187` reference with another
exact lookup. Use the source revision and record hashes to retain provenance
across steps. On 14 November 2026, the installed Codex allowlist enabled both
knowledge tools. Independent MCP SDK calls tested search and exact lookup from
the saved configuration. Existing connections can still need a refresh. See
[installed acceptance evidence](live-acceptance-2026-11-14.md).
