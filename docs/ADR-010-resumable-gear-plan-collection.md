# ADR 010: Resumable offline gear-plan collection

Date: 8 November 2026. Status: source/offline implementation.

The local plan journal records progress but previously required the caller to
manually collect every next window. A separate explicit CLI controller now calls
the existing offline ranker collector in process. MCP subprocess lifetime stays
with the SDK's stdio context, including cancellation. No simulation engine or
native game invocation is accepted.

The caller supplies a trusted absolute MCP executable, exact saved snapshot,
template, journal and slots. Candidate filters are explicitly enumerated. Window,
draft, invocation-time and payload budgets remain bounded. Collection resumes
the first reported gap and clips each request before a later saved interval.
Complete journals return status without recollecting.

An optional collection binding is stored inside each normal journal artifact,
transactionally with the first successful window. It hashes explicit selections,
template bytes/path, executable bytes/path, five controller/ranker/journal source
files, Python and MCP SDK versions, and the two supported data/catalogue environment
paths (plus file contents when applicable). It is rechecked before and after
collection. Budgets and page size are excluded so the caller can adjust them.
Changed bindings or existing unbound manual windows reject rather than being
silently adopted. Existing unbound manual artifact formats remain supported.

This fingerprint detects ordinary input drift; it is not authentication, a full
dependency/environment manifest, process attestation or protection against an
attacker modifying files while preserving metadata. Snapshot and catalogue
validation remain the MCP's responsibility. Configuration folder contents are
not hashed recursively. Updated bound code needs a new journal.

Journal operations share the invocation deadline, retain transactions and value
limits, and preserve committed windows on subsequent failures. Errors omit local
paths and raw MCP exceptions. The caller still chooses a private local journal
destination; Windows folder ACLs remain outside this qualification.

Reported coordinate coverage is not feasible-set coverage, simulation ranking,
complete ownership, full BiS or native qualification. Those remain separate work.
