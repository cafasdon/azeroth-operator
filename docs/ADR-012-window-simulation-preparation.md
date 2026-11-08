# ADR 012: Revalidate saved windows before preparing simulation requests

Date: 9 November 2026. Status: source/offline implementation.

Retained numeric gear material cannot replace equipment reviews or qualify a
simulation job (ADR 011). A separate explicit CLI now prepares one exact saved
window without invoking a simulator. It reads the selected journal row through
the existing read-only schema/digest/metadata limits, requires retained material,
and validates strict saved selection shapes before any MCP request.

It calls only `review_equipment_selection` on an explicit trusted absolute local
MCP executable for every resolved review, including baseline. It does not rescan
the candidate pool or recompute combinations. The source snapshot ID and offline
review flags/checks must agree. Fresh equipment must match both the retained
numeric specification and hash. Existing two-hand clearing behaviour is preserved.

The existing `request_from` adapter then checks captured level, template identity,
professions, Hunter pet and native slot conflicts. Unknowns retain that adapter's
existing semantics; preparation does not assert all character observations or
models are completely qualified. Missing material, changed selections/equipment,
unsupported observations or conflicts fail the entire preparation, with no partial
CLI request output. No omitted metadata is fabricated from the stored gear alone.

Templates/encounter inputs are strict bounded JSON. Consumed bytes must equal
their input fingerprints. Current executable/template/encounter/preparer/adapter/
catalogue files are fingerprinted before and after preparation. These identify
this new preparation, not the original collector environment or an engine.
They detect ordinary drift, not malicious file substitution or attestation.

The CLI has a 1–45 second checked timeout and a 4 MiB final-output limit, with
existing SQLite deadlines and SDK-owned stdio child cleanup. It neither modifies
the journal nor stores results nor executes engine commands. Request output
includes numeric gear and the supplied profile/APL; it is local private job data.
Snapshot character names and raw saved selections are not output, and the adapter
normalises the simulated player name. Errors omit paths and raw MCP exceptions.

An engine identity, execution, result journal, cross-window deduplication/ranking,
uncertainty, complete feasible coverage and full owned BiS remain separate work.
