# ADR 009: Local gear plan-window journal

Date: 8 November 2026. Status: source/offline implementation.

Coordinate windows and bounded artifact merging do not preserve progress across
process restarts. The companion now has an explicit-path local SQLite journal
for validated plan reports. It is separate from the character snapshot store
and adds no MCP, simulator, native client or remote service calls.

The version-1 journal has an application ID and exact schema. New/empty regular
files can initialise; foreign nonempty databases, unknown schema/version,
symlinks and nonregular targets reject before application writes. New files
use owner-only creation mode on POSIX, where existing files also require the
current UID and private permission bits. Windows uses the chosen folder's OS
ACLs; this implementation does not assert independently verified Windows ACL
isolation. The caller chooses an appropriate private local destination.

Appends are transactions with a five-second lock timeout. They validate the
strict plan artifact, compare its scope/baseline with the stored anchor, check
all indexed interval overlaps and enforce an explicit payload budget. Exact
canonical-byte retries are idempotent; conflicting retries do not overwrite.
Coordinates use fixed-width decimal keys to preserve large integer ordering.
The default payload budget is 64 MiB, configurable up to 512 MiB; it counts
stored report bytes rather than promising a physical database-file quota.

Status streams stored reports in a read-only transaction, rechecking content
digests, payload sizes, artifact validation, common scope and intervals. It
reports complete gap counts with at most 64 gap details. SQLite value/row limits
apply before retrieval, and SQL/streaming operations have bounded deadlines.
Original reports remain local payload data; status omits names, selections and
file paths. Checksums detect corruption, not malicious re-signing or provenance
authentication.

The journal persists conditional reported coverage. It does not schedule work,
execute saved commands, automatically collect the next window, merge simulation
rankings, prove feasible-set coverage or establish full owned BiS. Those remain
separate outcomes, as do native qualification and active-character detection.
