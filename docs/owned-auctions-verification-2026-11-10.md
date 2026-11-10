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
