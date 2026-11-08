# Character resolution — 8 November 2026

The read-only MCP tool `resolve_character` supplies a single explicit selection
contract without inventing active-player knowledge. Required `family` is one of
`forever`, `retail`, `classic`. Optional `character` requires name and realm.
Unknown arguments are rejected. Existing owner-store validation, case-insensitive
identity matching, capture-time ordering and hash tie-breaking are reused.

With an explicit target, the newest valid matching saved capture is returned in
`selection`, with `status: explicit_saved_character`. No matching capture gives
`no_matching_snapshot` and null selection. With no target, the response is
`active_identity_unavailable` and null selection; no saved-character scan occurs.
Every successful response has `live: false` and
`active_character_verified: false`. An explicit null character behaves as absent.

The tool selects history, not an actionable game session. It does not change
other tools' explicit snapshot-ID inputs, claim freshness, collapse Classic
variants into equivalent clients, or establish automatic active-character
detection. Client identity remains in the selected capture and must still be
checked for the task. No caller-supplied activity hint is accepted as evidence.

## Offline verification

- `cargo build --locked --offline`: passed.
- `PYTHONPATH=.;tests python -m unittest test_character_resolution`: one real
  stdio MCP test passed. It checks explicit selection despite a newer different
  character, case-insensitive identity, family isolation, owner-directory
  isolation, unknown default, missing matches, malformed arguments and route
  discovery. The owner-isolation fixture uses two independent temporary stores.
- No native identity signal, game action, installation or deployment was tested.

The existing Rust stdio protocol contract and both gateway-profile tests also
passed (four distinct targeted tests in total). The source profile now exposes
the route alongside the other read-only tools; no deployed profile was changed.
The test supports `AZEROTH_TEST_BINARY` and platform-specific binary names; its
explicit binary override was exercised on Windows. Linux execution was not run.

Independent Callers review reproduced the initially missing gateway/expected-tool
entry; both source contracts were corrected and its four-test rerun passed.
Failure/security review found a Windows-only test path; it was corrected to the
repository's environment/platform convention. These were integration/test defects,
not evidence of active-identity detection.

Final independent rounds were clean: Callers ran four targeted tests in
4.920 seconds; Failure/security ran the same four in 6.043 seconds. Both
used the explicit binary override. Failure probes also covered equal-time hash
ties, stale/corrupt/future captures, all-invalid stores, null targets and literal
traversal-looking names. Author's final four-test run passed in 6.338 seconds;
format and diff checks passed. Reviewers were `gpt-6-astra` at high effort.

Jev finding triage advisories, both `jev-1.14.0`:

- Gateway parity: request
  `3fd77eb204834caa205d7020a8c3e1109f36e3d807f259cd420f03b2f1eadd1e`,
  402 input / 47 output tokens, 0.278 seconds; fix-required probability 0.96.
- Test portability: request
  `6c088de6e923598f14acbf0224360c5c9b5df0fac007d70885b56635ba9a7e20`,
  387 input / 45 output tokens, 0.318 seconds; fix probability 1.0.

Jev plan advisory: resolved model `jev-1.14.0`, request
`23a83cc7bf64736a79e01df5ae30f70745e58cc6cf58ce39feba972167d3390e`,
470 input / 47 output tokens, 0.297 seconds; proceed probability 0.98.
This advisory is not verification of the source or a live qualification.
