# Typed visible loot — 2026-11-09

Previous goal turn made source progress as a76c46b. This unit expands loot system
coverage without game actions. The broad MCP/owned-BiS/native goal stays active.
Live game testing, installation and deployment remain deferred.

## Public source and author evidence

Pinned Gethe UI revision 9dc84f0d3bbae1768ac3405b3716000bd93eda06,
Interface/AddOns/Blizzard_UIPanels_Game/Mainline/LootFrame.lua, inspected through
the primary public URL and bounded local cache. File length 22,100 bytes;
SHA-256 3e0e992271a5271dab9e856946c28970de772bb146529676ccda2bb9bae085fd.
Type/enum use is at physical lines 92–97; quantity/currency tuple and subsequent container
conversion at 228–231 (also 477–479). Browser-extracted lines differ from the
cached file's physical line numbers. Generated LootDocumentation does not define these globals,
so it was not used to invent amount units or currency reward transformations.

- `cargo fmt; cargo build --locked --offline`: ordinary Rust MCP built, 9.83s.
- `PYTHONPATH=.;tests python -m unittest test_loot_slot_types test_loot_window test_loot_requirements`:
  3 tests passed, 2.353s. New actual stdio MCP checks include 22 malformed row
  shapes and two unsupported families, mixed/64-currency/64-money/zero cases,
  raw u64 totals, exact inspection, known non-item versus unreadable requirements,
  stock/recipe isolation, immutable source bytes and fixed-error privacy.

The public source was cached under the tool temp directory; an initial attempt
to print the one-line cached recursive tree produced excessive output. Subsequent
inspection uses bounded selected files and parsed JSON fields, not raw tree dumps.

## Advisory and implementation ownership

Plan Jev 1.14.0 request
4f4bcc8a50157ee4970565b0bcc1dce2ff34961a6459412bdc4b2a49be401ab4:
proceed .75 / revise .24 / insufficient .01; 599 input, 47 output, .291s.
Advisory only. Raw currency and absent money-amount interpretation deliberately
retain the source uncertainty. One scoped gpt-6-astra high writer owns only the
Lua collector/new Lua test; parent owns readers, analysis, MCP tests and docs.
Writer runs no builds or live actions. Test/package/review outcomes follow below.

## Lua, final readers and uninstalled package

- Initial new Lua run: 2/4 passed; two secret-enum scenarios incremented the
  fixture's global secretInspections count. The writer traced this to existing
  unrelated AO.collectors.spellbook: it dereferences Enum.SpellBookSpellBank
  before checking its API. The scoped fixture now excludes only that collector,
  keeps secretInspections==0 and labels setups; actual manual Core capture,
  equipment, encoding and strict import remain exercised. The unrelated source
  guard issue is recorded for follow-up, not claimed fixed by this unit.
- `cargo test --locked --offline --test loot_slot_types --test loot_window`:
  new tests 4 passed in .36s; existing loot tests 3 passed in .05s; compile 1.24s.
  Four new tests cover raw currency/zero/u32 limits, money with no info/link
  reads, unstable/restricted/malformed per-row and API/enum/context surfaces,
  identity spoofing, absent-enum legacy path and no forbidden actions.
- `cargo build --locked --offline --example verify_addon`: 1.71s.
  Final ordinary `cargo build --locked --offline`: 2.40s.
- Final scoped actual MCP reader/requirements regression: 3 tests passed, 1.772s.
- `python -m tools.package_addon --family forever --variant live --interface 16001 --version 0.48.0-dev --output dist/forever-typed-loot-0.48.0-dev`
  followed by `python -m tools.verify_addon_packages --directory dist/forever-typed-loot-0.48.0-dev --verifier target/debug/examples/verify_addon.exe` passed.
  Four-file capture-only package, exact rendered/source manifest checks and
  packaged Lua 5.1 roundtrip; Forever fixture and changed-interface refusal true;
  live_qualified false. Archive SHA-256:
  6b34d247b3766fbd4ca0fdbc9efb605b9b59482277ce4a6b62fa2616a0bd7060.
  Collectors SHA 8cd9f643ca15128176eeadcf7c3d44bf3640fe132be37a986e51b25a0132c476;
  Core unchanged 28f8eded6884753ce25ff2aa4ee08db3e803424cd2df093faf15d27eb401b03f.
  The package is not installed or deployed.

## Current-contract correction

Callers review found the original client README/interaction paragraphs still
stated the old item-or-unknown-only contract, conflicting with the new typed
shapes. Updated those current definitions; dated historical loot evidence stays
historical. Jev 1.14.0 triage request
5df59eb8f803ac68cad649218500eeade4257aeb75d01d9e2e9e1aeb11768046:
fix .98 / dismiss .00 / insufficient .02; 418 input, 44 output, .307s.
This was a documentation contradiction, not a runtime correction. New Lua fixture
secret-inspection assertions apply to the scoped loot collector plus Core, with
the unrelated spellbook collector explicitly excluded; they are not proof of
zero restricted reads across every source collector.

Second-round Failure review found another original requirement paragraph still
associated every non-item row with unknown item quantities, and browser-extracted
source offsets had been recorded as physical file lines. Author verified both
against src/analysis.rs/tests and the checksum-pinned local file, then corrected
the requirement paragraph/summary description and exact source offsets. No
runtime changes. Jev 1.14.0 triage request
6db7fe7e0a10a2c91becfed6ab08e293efd289bd079a707c47fe684c5250794b:
fix .90 / dismiss .01 / insufficient .09; 454 input, 44 output, .345s.

## Independent verification and final disposition

Two independent read-only gpt-6-astra high reviewers (Callers and Failure/privacy)
found no runtime blocker. Callers ran both existing Rust binaries (4+3 passed),
all three scoped Python/MCP tests (1.831s), traced import/storage/summary/stock
callers and independently checked the four-file source rendering. Failure ran
both Rust binaries (4+3), typed/legacy Python tests (2 passed, 1.047s), and 18
additional malformed raw hash-valid captures; Python and actual MCP rejected
them without private markers or paths. Both read the pinned primary file.

Three review rounds were needed for current documentation: the two distinct
contract paragraphs and source physical-line citations above were corrected.
The third round was clean in both lenses after exact readback; no runtime source
changed during those rounds, so tests were not needlessly repeated. Final source
hash/length was independently reconfirmed. Native/deployment drift lens omitted
because no installation or live testing was authorised in this unit. Agent token
usage unavailable.

Author rebuilt the ordinary binary after Rust checks; reviewed executable SHA:
d14159b17c52bfa81d773f1950b3b633244e7b47f53b68cd91d91b1350b87ba5.
Reviewers used that existing binary, without independently building source.
The separate pre-existing spellbook guard issue remains recorded for follow-up.

Final Jev 1.14.0 advisory request
52f5bf2c2cfe61ff8f9df7085c5250cc297c0c1eefdba3dc5c9a018d795c8f95:
land_bounded_unit .98 / revise .01 / insufficient .01; 625 input, 56 output,
.291s. Author accepts source integration on the concrete offline tests and
final clean independent round, keeping native gates and broad goal open.
Final cargo fmt --check and git diff --check passed.
