# Bag compatibility preflight — source-only

2026-11-04: BagCompatibility.lua adds read-only CheckBagCompatibility(link,bag), returning compatible/incompatible/unknown plus observed metadata. It is not packaged or connected to writes yet. Next integration must check both directions before swap pickup and destination before move/split pickup; unknown must refuse rather than bypass.

Requires Retail/current interface, no combat, explicit nonempty bounded link and carried bag bounds. Reads container capacity/free slots/family, item family/equip location/reagent flag and native reagent-bag classifier twice. Missing/invalid metadata, unavailable APIs, thrown/secret-value errors or changing observations return unknown. It never treats nil family as zero and never requests item use, pickup, network or asynchronous loading.

Eligibility interpretation: reagent bags require the item reagent flag; general mask0 accepts; container items INVTYPE_BAG are refused in specialised bags; other specialised bags require intersecting item/bag masks. This is an engineering eligibility check, not full native legality proof. A full bag can be compatible for swapping: free count is stability evidence, not a demand for vacancy. Actual GUID/location/cursor/readback checks remain necessary. Native special-case semantics and current-client compatibility remain unqualified.

References read2026-11-04: [native generated Item API](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua) confirms nullable GetItemFamily and GetItemInfo equip/reagent returns; [native generated Container API](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua) confirms capacity/free/family returns; [native ContainerFrame](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/ContainerFrame.lua) supplies ContainerFrame_IsReagentBag. These signatures/classification are not permission or end-to-end placement evidence. Downloaded research copies: local-data/bag-compat-research.

Tests cover general/full, reagent true/false, mixed/disjoint masks, container-item exception, missing item/bag family/reagent/API, changed metadata and carried boundaries. Full source gate passed: format/build/Rust-Lua/Clippy/38Python tests. No executor, deployed service or installed addon changed.

Jev advisory a4ace3b4d86601a1848b9ae6388eddcdc22ff1ef0c8d812f5e422b7cf33ca2ee, jev-1.14.0,1130input/154output tokens,0.292seconds: insufficient_evidence(probability0.51,confidence0.26); no concrete finding/pass. Configured adversarial-review file absent. Direct source inspection and fixtures establish only the stated read-only behaviour.


## Executor guards and package — 2026-11-04

Move/split now require compatible destination metadata before pickup; swaps require compatible metadata in both directions before pickup. Unknown/incompatible or missing helper refuses. Exact item identities/quantities/links/lock states and destination emptiness (for moves) are checked again after metadata reads, closing the observed preflight race. Existing cursor/uncertain-result behaviour remains unchanged.

Tests load the actual compatibility helper with mocked native metadata; normal move/split/swap and delayed completion fixtures pass. New zero-write cases include reagent destination rejection, unknown metadata, reverse swap direction rejection and GUID changes during metadata reads. Full source gate passed with38Python tests.

Retail0.20.0-dev packages BagCompatibility.lua before executors,36files total. Exact manifest allowlists retain prior35and older shapes. Bag preparation advertises0.20minimum for all three operation types. Staged swap lifecycle reads the actual packaged helper; existing action/talent/equipment staged fixtures also pass. Temporary upgrade from actual0.19archive verified36files without touching the game installation.

ZIP `dist/bag-compatibility-release/AzerothOperator-0.20.0-dev-retail-live.zip`, SHA256 `5f84162696f78e44469d5cdebcd5cfad3c5a851175d258532eb40b91b7dd339a`. Compatible service/companion/addon deployment remains pending; native qualification remains false.

Jev advisory `38bc226afc7bf636df4b6fffd867649fea47efacfa563eac5388da9c7c104057`, resolved `jev-1.14.0`,1077input/154output tokens,0.306seconds: insufficient_evidence(probability0.48,confidence0.22), no concrete finding/pass. Configured adversarial-review file absent. Direct source/fixture/artifact verification supports the stated offline evidence only.
