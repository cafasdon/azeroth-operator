# Captured Forever talent review

The default `captured_talent_loadout` endpoint now returns the validated saved
Forever trait allocation, rather than rejecting the entire client family.
The result has status `captured_allocation_for_review`. Its `loadout` includes
the exact captured configuration and nodes, plus observed tree/definition,
staged-change and unspent-point fields when present.

This is player-review data, not a native import string. Native import,
complete tree coverage, simulator mapping, build legality, game execution and
live status remain false. Missing talents return `not_captured`. Classic
retains the unsupported-family response. Retail and the optional explicit
Forever simulator comparison/preparation modes retain their separate behaviour.

Validation: six planning tests, four allocation tests and three definition
tests passed. Independent review exercised 15 additional real stdio cases,
including malformed, absent and staged observations. It found two stale MCP
test expectations; both were corrected and their focused recheck passed.
No implementation flaw remained. Installed endpoint evidence is recorded
separately in the pilot repair report.
