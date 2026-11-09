# Policy evidence and remaining goal checkpoint — 2026-11-09

Baseline 6466584. Previous goal turn made progress with historical reset
projection. Current source audit reconfirmed resolve_character without explicit
target returns active_identity_unavailable. Unified owned-gear evaluation still
reports complete ownership/feasible-set/global ranking/BiS unverified; its
reported-space completeness is narrower evidence. Native signal and gameplay
qualification remain explicitly deferred. Broad objective stays active.

## Published-source check

Read official opening posts, excluding community replies:

- UI addon policy: https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534/1
- Input broadcasting: https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613/1
- Third-party software, dated 5 August 2025: https://us.forums.blizzard.com/en/wow/t/prohibitions-on-third-party-software/2142972/1

These describe addon distribution/conduct, input mirroring and client/security
restrictions. They do not establish express project approval. Direct UK/EU EULA
2c72a35c-bf1b-4ae6-99ec-80624e1b429c and US EULA
bfbbb648-bcc6-4b78-a5c7-e2fe20c135df requests returned HTTP 403. Search snippets
were not treated as complete current account/region terms. No full text copied.

## Implementation and direct evidence

Capabilities retain enforcement mode/default/owner scope/legacy sources and add
explicit false authorisation, full-terms-review and absolute-compliance flags.
The updated date is labelled published-source checks and project controls only.
Five structured references expose retrieval outcome/date; no runtime fetching,
new allowlist entry, addon change or installed/hosted deployment occurs.

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_policy_evidence test_mcp.MCPTests.test_snapshot_owner_scope
```

Ordinary build passed, 5.60s. Two actual stdio MCP tests passed, .477s. The new
case checks evidence/claim boundaries, actual advertised tool names, and direct
calls to seven legacy preparation routes plus equip/cast/script/chat routes:
all unavailable. Empty store stays unchanged. Existing owner-scope test also
passes. No game, credentials, personal snapshots, CI or full-suite replay used.

## Advisory plan

Jev resolved jev-1.14.0, request
`b403d508a75adf1d9e3b24ba4edc012941626552f3f167afb9beee20e6b961f2`:
proceed .99 / revise .01 / insufficient .00, confidence .98; 516 input /47 output
tokens, .307s. Advisory only; this is not a legal approval or permission grant.

## Independent reviews

Two independent read-only gpt-6-astra high reviewers (runtime model IDs and usage
not exposed) ran the new actual-MCP test, 1 passed each (.382s/.207s). Callers
also checked exact 11/13 allowed-route sets with catalogue absent/present,
sorted capabilities/list parity and the tool description's approval disclaimer.
Failure/privacy made six capabilities calls across two synthetic owner stores;
injected approval/execution/owner arguments changed no fixed metadata. No private
canaries appeared; stored bytes unchanged. All 11 direct-call denials passed.
Both documentation readbacks were clean; no confirmed fault. Neither reviewer
made network requests or claimed independent web provenance/legal authorisation.
No reviewer builds, source writes, Git mutations or live actions. No live-drift
pass because installation/deployment remain deferred. Final format/diff checks
passed. Ordinary MCP SHA-256:
`3d00503cfc08b3975efd937d87e719914ed93a4d20ef5d3fbe4254fc422e1b61`.

Final advisory Jev resolved jev-1.14.0, request
`3eb9495b16d65b9f6128ad77a70f08ad885de5c0a8e146b034605df78836142c`:
integrate .70 / revise .28 / insufficient .02, confidence .55; 528 input /52
output tokens, .355s. This is uncertain advice, not an approval gate; it was not
rerolled. Author integrates only the metadata clarification on actual route
checks and clean independent review. Incomplete terms/authorisation remain
explicitly false and no restriction is relaxed. Broader completion is not claimed.
