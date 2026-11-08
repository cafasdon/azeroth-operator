# Native item-stat observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever item documentation](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
defines `C_Item.GetItemStats(itemLink)`, which may return nothing. The manual
Forever collector now optionally adds `native_stats` to linked equipment and
container-item observations. The existing container helper also serves visible
character/account bank captures. Each read checks the link's reported base ID
with `GetItemInfoInstant` and passes the exact full link to `GetItemStats`.
No explicit item load, tooltip query, equipment change or network sync is added.

Only numeric maps with at most 128 fields are accepted. Keys are at most 96
ASCII bytes, shaped as `ITEM_MOD_[A-Z0-9_]+_SHORT`, `RESISTANCE[0-6]_NAME`
or `EMPTY_SOCKET_[A-Z_]+`. This is a bounded token-format contract, not an
exhaustive list of recognised gameplay semantics. Values must be finite within
plus/minus u32 maximum, including fractional and negative modifiers. Two
complete readable maps must agree. Restricted table/key/value observations,
errors, nil returns, unsupported keys/types, invalid bounds, changing maps
and base-ID mismatches leave the optional map absent while preserving valid
item identity and unrelated metadata. Private error text is never exported.

An empty returned map is preserved as an empty observation. It does not prove
an item has no stats or that data loading/effects are complete. Missing tokens
are not assumed zero. Native values do not establish all affix, enchant,
set-bonus, proc, socket or scaling effects, nor simulator compatibility.

Both readers validate the optional non-null map for all supported inventory
storages, require a nonempty item link and enforce identical token/numeric
bounds. The generic item contract can carry these observations across client
families; new native collection is enabled only for Forever. New exports
require updated readers before syncing. No existing client is newly qualified.

Each equipment-review change adds `native_stat_comparison`, null when either
item lacks the map (or no prior item was observed). Otherwise, `shared_tokens`
reports before/after values and their raw delta. `observed_only_before` and
`observed_only_after` retain values without fabricated deltas. These are
per-item token observations in one snapshot; they are not a full-set sum,
legality check or an engine input conversion. `effect_on_performance` remains
`unverified`. Existing catalogue/simulation checks are unchanged.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test native_item_stats --test item_required_level --test addon
cargo test --locked --offline --lib planning::tests
$env:PYTHONPATH='.;tests'
python -m unittest test_native_item_stats test_native_level_requirements
cargo fmt --check
git diff --check
```

Build passed; eight Rust tests and two real MCP Python tests passed. Actual
Lua export/Rust import covers equipment and bags, negative/fractional/zero
values, empty maps, missing APIs/links, ID mismatch, restricted keys/values/
table, errors, nil, excessive size, invalid keys/types/nonfinite/overflow and
changing/added tokens. Real MCP cases exercise all inventory readers,
equipment review with shared and one-sided observations, nullable legacy
comparison, empty maps, and fixed rejection diagnostics. The initial bank
fixture reused one container across two storage sections; the existing Rust
guard correctly rejected it. The fixture was corrected to distinct containers
without weakening the guard. Native required-level/ranker coverage remains
passing with the pinned local catalogue; no automatic downloads occurred.

Development package, not installed:
`local-data/native-stats-qualification/AzerothOperator-0.0.0-native-stats-test-forever-live.zip`.
SHA-256: `d6aaaec15b5af557a0cf812eff5cfc83da32080907edc2c221a5e90a19624caf`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no actionable defect. Both executed the prebuilt Lua/Rust
fixture. Callers rejected 36 malformed cases across four storage types and
accepted five valid boundary/empty cases; failure review passed 84 in-memory
boundary probes including enormous integers, nonfinite values, null, booleans,
Unicode/newline/overlong keys and map-size limits. Callers traced catalogue
enrichment preserving the comparison and simulator consumers continuing to
use link/catalogue resolution. Both checked validation precedes comparison
unwraps and one-sided observations receive no invented delta. They inspected
but did not run temporary-store MCP tests under their read-only interpretation;
the author ran those against the rebuilt current-source server. No review
edits, compilation, cleanup, deployment or live operations were performed.

Jev advisory supported offline source landing: `jev-1.14.0`, request
`37b380509aebd0db3f3de5e817f59cadba4b780c25dfeebba74f6831c3b5cbda`,
917 input / 52 output tokens, 0.290 seconds. Probabilities were 0.97 landing,
0.02 needs fix and 0.01 insufficient evidence. Actual caller tests and source
review, not advisory confidence, establish the scoped verification.
