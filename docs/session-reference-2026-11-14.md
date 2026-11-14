# Exact session export selection

`resolve_character` accepts optional `current_snapshot_id` when the caller has
identified an export from its current game session. An explicit character
always overrides the reference. Otherwise the MCP reads that exact hash from
its configured owner store and requires a non-future capture less than 15
minutes old. Family, exact client, timestamp bounds and required sections must
match. The two latest-capture options are incompatible with an exact reference.

The result uses status `current_session_snapshot_reference`, basis
`caller_current_session_snapshot_reference`, age and capture-derived expiry.
Retries never renew expiry. Invalid, missing, corrupt, other-owner, expired,
future or mismatched references return the same bounded public error; none
selects another snapshot. No mutable identity state or new transport exists.

This is a useful exact selection for a caller that just observed and exported
the current character. It does not establish automatic detection. Both `live`
and `active_character_verified` remain false. Without either selector the
existing unknown-identity result remains. Native SavedVariables persist only
after reload/logout; inspecting those files cannot prove continued play or
detect a later character switch. No process memory inspection is introduced.

Checks: fixed-clock Rust expiry/future-boundary test (1 passed), real stdio MCP
character resolution tests (2 passed), independent owner/filter/error probes
and an independent security/failure review passed. The review found no medium
or high issue. Installed real-character readback is recorded separately in
the pilot repair report.
