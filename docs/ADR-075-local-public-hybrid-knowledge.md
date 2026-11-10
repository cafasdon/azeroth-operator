# Local public hybrid knowledge

The public reference catalogue now has a derived local SQLite index.
FTS5 keyword ranking and exact cosine search over 384-dimensional MiniLM
vectors are fused with reciprocal ranks. Exact source identities remain
available for the existing catalogue and owned-inventory tools.

The corpus covers the pinned public items, consumables, NPCs, zones,
factions, enchants, random suffixes, spell effects and encounter presets.
Character snapshots are outside this index. Both source
and model are checksum-pinned. Model execution uses local files, disabled
network lookup and no remote model code. Explicit sync commands download
public inputs. Build creates a complete temporary index and replaces the
old file atomically. Identical input records and model metadata reuse the
existing index. Changed pins cause a rebuild.
Reuse validates keyword content and normalised vector shapes as well as
source records. Recognised indexes with damaged derived content are rebuilt.
The derived integrity checksum includes vector blobs, the SQLite schema
and all FTS shadow tables, including content row identities.
Unrecognised or malformed index identity metadata is rejected, preserving
the configured file. Additional model assets are rejected before loading.

The optional stdio MCP companion exposes one read-only search tool.
Paths are startup configuration, and queries cannot select files, models,
network endpoints or snapshot stores. Retrieval returns candidate record
IDs, names, provenance and retrieval scores. These scores do not estimate
gear quality or establish any missing game facts.

The current corpus has names and catalogue source references. Rich quest
text, guides, additional world records and user-specific text require
separate source/licence, version and owner-scope ingestion decisions.
The companion is an optional process. It is registered locally on ICARUS
as `azeroth-knowledge`, with one enabled search tool, a 60-second startup
allowance and a 30-second tool timeout. Hosted deployment remains deferred.

Validation performance follow-up (10 November): every query still verifies
source records, visible FTS rows, derived vector/schema/FTS seal, individual
vector dimensions and finiteness. Individual float32 vector norms are computed
as before, then their tolerance comparisons use one NumPy array operation.
Tolerance and retrieval calculations are unchanged. No index format, query API,
model, data source, persistent query cache or privacy boundary changes.

A twelve-pair alternating offline benchmark on this host recorded warm lookup
medians of 0.667274 seconds before and 0.412818 seconds after (38.1% reduction).
The same warm model and pinned index were used, startup excluded, responses
identical and index unchanged. This small fixed-query result is not a general
performance, relevance, simulation or BiS guarantee. See the recorded benchmark
and verification evidence for inputs and acceptance-boundary tests.

Direct public links (10 November): search_forever_knowledge accepts Boolean
include_references. It implies the existing base source row/hash and requires
limit 1..5. Each hit gains catalogue_references with recognised direct fields:
top-level zoneId and npcId/zoneId/repFactionId inside source objects. These
join only to the same pinned catalogue's npcs, zones and factions. This is not
a recursive graph, name-based match, route calculation or external enrichment.

Each edge retains its RFC6901 source_path into the attached base row (zero-based
source array index), target section and record ID. A resolved edge includes
exact target row/key/canonical SHA256. Missing rows are not_in_pinned_catalogue,
not proof that an NPC/zone does not exist. Invalid IDs are explicitly marked
invalid_reference_id with null record ID; the base row retains the source fact.
Other source relationships, such as quest reward IDs, are not represented as
missing supported targets. IDs stay in the catalogue's namespace; native world
identity, current acquisition availability and full graph coverage stay false.

The response reports recognised edge count and returned count, with at most
32 edges per hit and explicit truncation. Counts are edges, not distinct entities;
several source paths can reference the same target. The existing 256 KiB bound
covers base and related records together. Returned rows are copies, not aliases
into the process's verified public source map. Default queries and source-only
queries keep their previous response shape. --include-references is the CLI
flag. No index rebuild, model change, private snapshot access, network lookup,
persisted query data, installation or native operation is introduced.

The pinned corpus has 4,124 recognised edges across 7,749 records: 2,990 resolve
and 1,134 lack a target row (882 NPC and 252 zone links). Maximum per record is
22, so the current corpus is untruncated. This measures recognised local links
only, not complete world/acquisition coverage.

Suffix-option extension (10 November): recognised direct links also include
randomSuffixOptions array entries pointing to random_suffixes records. Each
edge is labelled catalogue_option_not_owned_variant; owned_variant_verified
stays false. These are listed catalogue possibilities, not proof of an owned
roll, effective item stats, acquisition availability, simulator resolution or
best-in-slot eligibility. No native/gear-planning state is inferred or changed.

To reach every edge while retaining the 32-edge response bound, include_references
accepts reference_offset (uint32, default zero; CLI --reference-offset). A nonzero
offset requires include_references. The offset applies to every returned hit's
recognised-edge list. Existing world links retain their order, followed by suffix
options in source-array order. Each hit reports reference_offset, reference_limit
32, total/returned counts and has_more_references. Advance by 32 while has_more
is true, keeping the same query/filter/base record key/hash and pinned source.
Beyond-end offsets return an empty edge page with the full count and no more.
references_truncated means the current response omits any recognised edges,
including an earlier prefix; it can remain true on the last page. It is not the
iteration condition. No recursive traversal or name matching is introduced.

Earlier counts above describe world-field links only. Adding 30,493 suffix-option
edges for 960 items yields 34,617 recognised edges. All suffix targets resolve;
the previous 1,134 missing world targets remain explicit. Maximum per record is
43; 575 records require a second page and every current record fits two pages.
The combined 256KiB bound and source/target hashes apply to each response.
No index rebuild, model change, private-data copy or network lookup occurs.
