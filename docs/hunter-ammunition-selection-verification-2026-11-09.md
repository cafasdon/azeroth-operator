# Hunter ammunition preflight — 2026-11-09

Baseline3fba026. Previous goal turn made progress integrating quiver diagnostics.
This unit fixes the analogous unknown-ammunition fallback and exposes its
remaining template assumptions. Broad MCP/full-owned-BiS/native/fresh-active
objective remains active. No live client, account data, installation or deployment.

## Reproduced failure and source

Local pinned9dc32ac8ca0d8d1b76337861d38b93c0f884af11 proto/hunter.proto defines
nine Ammo choices plus none. Generated sim/hunter/ammo_auto_gen.go supplies item
IDs and DPS. NewHunter reads arrows[Ammo.String()].DPS; an unknown numeric enum
has no map entry and therefore zero-valued DPS. Hand-built ammo999999 passed
request_from. New regression expected ValueError and failed (.001s). Minimal
pinned-reference validation now rejects it before review/preparation execution.
Supported/default/null requests remain unchanged; physical ammo remains unknown.

## Author checks

```powershell
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
$env:AZEROTH_TEST_SIM_SOURCE_ZIP='C:/Temp/codex/forever-sim-9dc32ac/source.zip'
$env:AZEROTH_TEST_REAL_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
$env:AZEROTH_TEST_PUBLIC_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
python -m unittest test_hunter_ammo_selection test_hunter_quiver_observations test_merge_prepared_windows.MergePreparedWindowTests.test_context_hash_source_row_and_count_rejections
```

Nine scoped tests passed,1.773s, including explicit source ZIP and qualified real
engine. ZIP SHAe1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712
and all ten enum IDs/nine generated reference values agree. Limited real Hunter
runs compare AmmoNone and ThoriumHeadedArrow at200iterations/45 simulated seconds,
seed20261109; arrow selection yields higher mean DPS. Engine hash remains
833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983.
Quiver real-reference checks also remain green in that scoped invocation.
These are plumbing examples, not native-effect or optimality qualification.

Strengthened the bad-selection test to invoke production main() with real JSON
files, engine spy and a missing checkpoint directory. Ten malformed selections
fail with exit1, no stdout/private echo, no engine launch and no directory creation.
That test plus prepared merge mutations passed2tests,.306s. The fresh-hash invalid
ammo case additionally asserts the specific enum rejection instead of relying
on unrelated context validation:1testpassed,.077s.

Actual MCP multi-page saved-journal test passed1test,41.259s, retaining ammo
reference ID0/DPS0 with false identity proof across two encounters, top1,
partial work and checkpoint reuse. It uses a controlled engine subprocess double
for exact ranking; separate real-engine tests supply limited execution evidence.
No CI, full-suite or unrelated broad test run.

## Advisory plan

Jev resolved jev-1.14.0, request
ee6a3cd9d2f641d58b4bf887de6cb6d874533203b6f592d300f059aa229b4133:
proceed.82/revise.16/insufficient.02, confidence.73;609input/47output,.372s.
Advisory only; direct failure reproduction and independent review decide scope.

## Independent reviews

Two independent read-only reviewers assigned gpt-6-astra high (runtime IDs/usage
not exposed) found no concrete defect. Callers ran four cheap helper/request/
CLI/merge tests,.157s, independently verified the pinned ZIP hash, all ten enum
values and nine generated references, and exercised60 name/numeric/capture
combinations. Captured ID coincidence never upgrades ammo proof; requests and
inputs unchanged. Failure/privacy ran four scoped tests,.156s, plus120 valid
helper/request cases across both class-option aliases and missing/empty/
coincident-ID observations. Another24 malformed/private/nonfinite enum values
reject, and five protobuf-null/default cases stay unverified. No private echoes
or input mutation. No reviewer engine/heavy-suite reruns, builds/source writes/
Git changes/network/game/deployment. Both documentation readbacks were clean.
Live drift omitted because installation/native testing remain deferred. Final
whitespace check passed. No compiled MCP/addon artefact changed in this Python
preflight unit; existing adapter fingerprints cover the new inline reference.

Final advisory Jev resolved jev-1.14.0, request
e5563061da6fbcb7b3ae8c997592451076aa3a8c312e897d7fd8f98eb029d4f6:
integrate.91/revise.06/insufficient.03, confidence.86;616input/52output,.286s.
Author accepts this scoped preflight/reference correction on source, direct
red/green, limited engine and independent evidence. Complete ammunition/native/
owned-BiS requirements and broad goal remain active and unproven.
