# Passive owned-auction observations — offline verification, 2026-11-10

Scope: ADR-077. Manual capture only, already-open auction panel, loaded owned
cache only, numeric allowlisted fields. Both readers and known-section coverage
registries updated. No installation, deployment or live game testing performed.

Author checks:

- cargo test --test owned_auctions -- --nocapture: two passed. Covers real Lua
  5.1 Core export through Rust import, stable/empty/256-row captures, exact
  maximum auction IDs, duplicate IDs, restricted/malformed/changing rows,
  hidden panel, replaced API, no queries/actions and no private sentinels.
- cargo build --bin azeroth-operator: succeeded.
- PYTHONPATH=.;tests python -m unittest test_owned_auctions: one actual Rust
  stdio MCP test passed in 0.545 seconds. Shared Python and Rust validators
  reject malformed maps, noncontiguous slots, private extras, missing fields,
  wrong families, duplicate IDs and invalid numeric bounds/types. The foreign
  owner store is inaccessible. Auction quantities do not increase bag stock.
  All synthetic store files remain byte-identical.
- cargo test --test addon -- --nocapture: two existing roundtrip and Forever
  capture tests passed after the shared numeric encoding change.
- Temporary four-file Forever live/interface16001/version0.0.0-dev package:
  SHA256 4e05774970be2ad2b503c2cb0969871f3aa82f59da449a2fb123894f86f4fa00.
  Actual verify_addon example passed packaged Lua 5.1 roundtrip, existing
  Forever capture fixture and changed-interface refusal. Package and staging
  were automatically removed. This is synthetic packaging, not native proof
  or an installed/versioned development release.

Author's first maximum-ID roundtrip exposed the Lua tostring precision/exponent
boundary and the schema's uint32 helper mismatch. Core now formats JSON-safe
integers with zero fractional digits; the new auction_id field uses a bounded
safe_uint kind. Rebuilt scoped tests passed; no rounding fallback is accepted.

Independent adversarial round: failure and security/data plus callers lenses,
requested gpt-6-astra at high effort. Resolved model and token usage were not
exposed by the collaboration tool. Both code reviews found no concrete defect.
Failure review independently exercised 20 secret/invalid/context mutation cases,
exact maximum integer transport, and ten JSON boundary tokens against both
validators and real binary import. Float/exponent/overflow/boolean/fractional
ID forms reject. Callback probes replaced frame/API/count/guard/family,
changed auction IDs and closed the panel. Source remained unchanged.
Security/data review checked real MCP isolation, private-field rejection,
exact uint53/u32 transport, all three section statuses, aggregate capture
coverage and combined section-status filtering. Source tracing found inventory
and wallet consumers explicitly scoped to existing owned storage/wallet only.
Auction data is neither a public knowledge input nor an inventory/copper source.

API/frame/UI declarations were checked from mirrored Blizzard Forever source
on 10 November, links in ADR-077. These describe getters and data types only;
installed-client availability and policy authorisation remain unverified.

Final documentation review found no false source/native/privacy/policy claims.
Advisory shipping judgement selected ready with low confidence 0.35 (ready
probability 0.57, insufficient_evidence 0.42), Jev 1.14.0, request
7ff6e6e01df93af073fe064198657bdfb939626b6a9cee85f722ba1ebc031ae4;
623 input and 44 output tokens, 0.300 seconds. This does not qualify a live
feature or resolve native/policy uncertainty. The author accepted the bounded
source-only change based on direct offline tests, packaging verification and
both independent reviews, keeping installation/deployment/native gates parked.

## Saved auction discovery and inspection — 2026-11-10

Existing list_snapshots gains owned_auction same-row exact predicates;
inspect_snapshot gains owned_auction_filter with bounded pages. No collector,
transaction, deployment or native test changed. See ADR-077 for the contract.

Author: cargo build --bin azeroth-operator succeeded. With PYTHONPATH=.;tests,
python -m unittest test_owned_auction_filters test_owned_auctions passed two
actual Rust stdio MCP tests in 1.402 seconds. Cases cover latest empty capture
suppression, contradictory same-row predicates, uint53/u32 maxima, raw status
zero, full 256-row aggregates, pre-page totals, numeric slots, huge offsets,
missing/empty distinction, default raw inspection, malformed page/filter input,
private canary redaction, foreign-owner/family refusal and no bag-stock credit.
All synthetic snapshots remained byte-identical. git diff --check passed.

One independent adversarial round: failure and security/data plus callers
lenses; both requested gpt-6-astra high. Resolved model and token usage were
not exposed. Neither review found a concrete flaw. Failure reviewer independently
passed 180 paging/count/unit/same-row cases, 82 malformed/redaction cases and
latest missing-section suppression, using the real MCP binary. Security/data
reviewer passed 36 independent MCP probes of schema, null/default compatibility,
malformed private input/snapshots and coverage conflict; traced root containment,
family restrictions and latest-before-filter order. Both independently ran the
new real-MCP test. No new inventory, wallet, public-index or execution input
was introduced. Documentation matched the historical-cache boundary.

Advisory shipping judgement: Jev 1.14.0 selected ready with confidence 0.83,
request df5ab1e7b4eea9f9bb35a3fc890e6dc51a2e0b8b548ae6c8930f1846fc83705c;
570 input and 44 output tokens, 0.317 seconds. Author accepted the bounded
source change based on direct tests and both clean independent reviews.
Native availability/ownership/completeness remain unqualified; live testing,
installation and deployment remain deferred.

## Saved auction comparison — 2026-11-10

Optional owned_auction_changes reuses the existing compare_snapshots route and
identity/client/time/owner gates. Numeric-ID joins ignore slot ordering. The
contract retains missing-side uncertainty, observed-empty zeroes, complete
category counts and bounded details; it never infers transactions or credit.
See ADR-077. No collector, installation, deployment or native testing changed.

Author: cargo build --bin azeroth-operator succeeded. With PYTHONPATH=.;tests,
python -m unittest test_owned_auction_compare passed one actual Rust stdio MCP
test in 0.747 seconds. Cases include changed item/quantity/raw status on common
IDs, disappearance/appearance, slot reordering, both/one missing cache, empty
maps, safe maximum IDs, 256-row sums and 50-row detail limits, negative deltas,
strict/redacted flags, default equality and foreign character/owner/family
refusal. All synthetic snapshot bytes remained unchanged. Diff check passed.

One independent adversarial round, failure and security/data plus callers
lenses, requested gpt-6-astra high; resolved model and usage not exposed.
No concrete findings. Failure reviewer passed 100 seeded comparisons against
an independent Python reference, including 49/50/51/255/256-row boundaries,
item-ID conflicts, reordered rows, missing/empty, safe/u32 bounds and signed
aggregates. Seventeen further flag/time/client/character/default/self checks
passed. Explicit null flags reject. Security/data review passed seventeen fresh
MCP probes covering exact build/variant/interface/name/realm/time gates,
malformed private snapshots/flags, missing observations, negative quantity
delta and unchanged defaults. Both independently ran the real MCP test and
checked the docs against the historical cache contract. No game, network or
deployment actions occurred. Numeric equality is not native listing identity;
aggregate unit changes are not sale, stock, delivery or currency evidence.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.92,
request 426f5c1470b1b87e2fd4debe5be0352438cda45eb35e5e98c7d89ef5ba493683;
612 input and 44 output tokens, 0.351 seconds. Author accepted the bounded
source feature on direct tests and both clean independent reviews. Native
qualification and live testing remain deferred.
