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
