# Equipped bag item observations — 2026-11-09

Baseline ebd0f03. Previous goal turn made progress with recipe capacity ordering.
Current source audit found template Hunter quiver/ammo bonuses not grounded in
captured bag identity. This unit closes the observation prerequisite only;
quiver/ammo preflight, complete owned BiS, fresh active identity and native
qualification remain open. Broad goal active. No live client/account data,
installation or deployment used.

## Primary source

Read pinned generated ContainerDocumentation.lua at Gethe
9dc84f0d3bbae1768ac3405b3716000bd93eda06, including nullable
ContainerIDToInventoryID return. SHA-256:
dd3e4741d45720384f814d3f78ce975dab3951458ae894e728ed3aedc8ae56b3.
Local pinned simulator source9dc32ac8ca0d8d1b76337861d38b93c0f884af11,
proto/hunter.proto and sim/hunter/hunter.go, explicitly selects Ammo/QuiverBonus
and modifies BonusRangedDps/RangedSpeedMultiplier. This is source evidence,
not a claim that any particular template is wrong or native effects qualified.

## Author direct checks and corrections

```powershell
cargo test --locked --offline --test equipped_bag_capture
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_equipped_bag_identity test_recipe_capacity_order
```

Initial2 Lua5.1 tests passed (.01s, compile31.38s); existing Core export1passed
(.35s, compile2.58s); ordinary build12.20s. Expanded whole Core fixture captures
and decodes item8217/slot20, with unchanged carried-space calculation:1passed,
.36s after3.78s compile, ordinary rebuild3.16s. Python2tests passed1.213s.

Author then hand-built a same-capacity free/family change during identity reads:
red regression failed (compile2.70s/runtime.01s). Added only free/known-family
rechecks so an identity cannot join changed base metadata. Expanded3tests green,
.01s after.68s compile; Core1green.31s; ordinary/example builds2.56s/2.67s.
A second concrete regression changed the item getter during the final free read;
red failed after2.52s compile. Moved the existing final pointer checks after all
final getters. Final3tests green.01s after.75s compile; Core1green.29s; ordinary/
example builds2.59s/1.76s. No guard relaxed or unrelated collector altered.

Final actual-MCP2tests passed.723s. Tests cover optional nil/secret/error omission,
required API secret guards without restricted access, ID/mapping changes,
duplicate mappings, base count/free/family stability, final getter replacement,
raw inspection/summary,17 malformed cross-reader cases, numeric limits, missing
versus no-identity captures, foreign owner refusal and unchanged bytes.
Item lookup receives no stock/location credit from equipped bag metadata.

## Package

```powershell
python -m tools.package_addon --family forever --variant live --interface 16001 --version 0.53.0-dev --output dist/forever-equipped-bags-0.53.0-dev
python -m tools.verify_addon_packages --directory dist/forever-equipped-bags-0.53.0-dev --verifier target/debug/examples/verify_addon.exe
```

Uninstalled archive: dist/forever-equipped-bags-0.53.0-dev/AzerothOperator-0.53.0-dev-forever-live.zip.
ArchiveSHA25650897dbcb4d34aab4438158c42de6d3c1fcde7bff23b13e6dcade16599bb8fae.
Packaged LF CollectorSHA25602fcaa16e0dd10b82b4369b669f0068f4f14a7ef230b0fad3c5da1b925971dbf.
Core unchangedSHA2560be2cdab8a33db1d1f83595ed2a87b2f655e1cf693226e05ade929b449f62157.
Exact source rendering/checksums, generated adapter/Lua5.1 roundtrip and changed
interface rejection passed. Packaged fixture omits optional identity when its
API is absent; identity-present capture is proven by the separate Core fixture.
No full-suite/CI/native test run.

## Advisory plan

Jev resolved jev-1.14.0, request
b814fbd35599e5edd00aa0cde02cbd6b0cda3e8cfa03e2f4713148f83ae64f08:
proceed.96/revise.03/insufficient.01, confidence.94;612input/47output,.291s.
Advisory only, not native/model or policy authorisation.

## Independent finding and minimal correction

Both read-only reviewers were assigned gpt-6-astra high (runtime IDs/usage not
exposed). Initial Callers checks passed3 Lua tests,1 Core fixture and actual-MCP
test1 (.409s), then independently proved five identical item IDs in five distinct
native slots of a six-bag capture are valid. No identity receives find-item,
inventory-discovery or recipe-stock credit under either material scope.

Failure/privacy found one concrete ambiguity: bag1 maps20 and reads item8217;
bag2 also maps20 but its item read fails. Tracking only successful pairs retained
bag1 incorrectly under the stated duplicate-mapping boundary. Supplementary
Lua5.3 reproduced it. Parent added the primary Lua5.1 regression first, which
failed (compile2.61s/runtime.01s). Minimal correction registers each valid mapping
before its optional item read and suppresses identities for every implicated bag,
even if the later item read returns nil/secret or throws. No base capacity or
schema bound changed. Four Lua5.1 tests then passed,.01s after.86s compile;
Core1passed,.36s; ordinary/example builds2.59s/1.78s. Final actual-MCP2tests
passed,.688s. Regenerated/reverified the uninstalled package; current hashes
above supersede initial archive9942a8b458057938f343f7f561668904d95f36066ec071f2de04062bd5e2267a.

Failure's initial independent39 domain/Python/Rust-MCP cases also passed for
pair presence/null/types/ranges, duplicate slots, repeated items in distinct
slots and private fixed errors. Bytes unchanged. No other fault found in that
round. Correction round and documentation readback are recorded below.
Finding triage Jev resolved jev-1.14.0, request
31e63666228c04b88e5632878cca8248418501aac2166610d48fb4c7f0fa48b9:
fix.96/insufficient.02/not-material.02, confidence.94;512input/45output,.302s.
Advisory only; parent reproduced and fixed the concrete primary-Lua fault.

## Clean correction round

Both independent read-only reviewers found no new fault. Callers ran the final
prebuilt primary Lua5.1 suite:4passed, including nil/secret/throw after duplicate
mapping. It traced mapping registration before item reads, same-bag repeats,
capacity preservation and unchanged stock/simulator paths, and independently
verified the final archive hash. Failure/privacy ran4primary tests plus11
supplemental Lua5.3 cases: failures on first/middle/last bags, sticky ambiguity
across three bags, changing mappings and valid unique pairs. Restricted accesses
stayed0 and base counts/free slots stayed intact. Lua5.3 supplements only.
Both documentation readbacks were clean and matched chronology/hash provenance.
No reviewer builds/source writes/Git changes/network/game/deployment. Live drift
omitted because installation/native testing remain explicitly deferred.
Final format/diff checks passed. Ordinary MCP SHA-256:
29a815754971d624d0703890f6dc846e93ed43a760c465df3234b21ee5cafc15.

Final advisory Jev resolved jev-1.14.0, request
d5dbe6eb02fb797a8ff6a036245e2630c69781124a81ff21cbc55e7c73f0cc06:
integrate.95/revise.04/insufficient.01, confidence.92;613input/52output,.367s.
Author integrates the observed-identity prerequisite on direct red/green and
clean independent evidence. Simulator quiver/ammo integration and full objective
remain unproven and active; no native or policy authorisation is granted.
