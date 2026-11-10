# Player life-state flags — offline verification, 2026-11-10

Scope ADR-079: manual out-of-combat Forever export, four raw player-only Boolean
getters, stable two-pass projection. Strict readers and section registries
updated; no action, classification, monitoring or public-index path added.
Installation, deployment and live game testing remain deferred.

Author checks:

- cargo test --test player_life_state -- --nocapture: two passed. Actual Lua5.1
  Core export through Rust import preserves all sixteen raw flag combinations,
  including explicit false and uninterpreted conflicting combinations. Eight
  player-only reads occur per successful capture; extra returns are discarded.
  Invalid/restricted/missing/changing reads and replaced APIs/context refuse
  the section. Combat and malformed global guard refuse the export.
- An initial test expected section-only refusal for a malformed global guard;
  existing Core correctly refuses complete export. The fixture expectation was
  corrected; no runtime fallback or unrelated Core behaviour was changed.
- cargo build --bin azeroth-operator: succeeded.
- PYTHONPATH=.;tests python -m unittest test_player_life_state: one actual Rust
  stdio MCP test passed in 1.232 seconds. Both readers reject private extras,
  missing/null/non-Boolean fields and wrong families. All sixteen combinations,
  absent-section handling, observed coverage and foreign-owner exclusion pass.
  Synthetic snapshot files remain unchanged.
- Temporary four-file Forever live/interface16001/version0.0.0-dev package:
  SHA256 aa5728b4c76941f909dc50411b68746134c50a70161dae1fdb16d8ac97c1e73b.
  Actual verify_addon passed packaged Lua5.1 roundtrip, existing Forever capture
  fixture and changed-interface refusal. Package/staging automatically removed.
  Generic fixture is not native life-API proof; separate source fixture covers
  life-state projection.
- git diff --check: passed.

One independent adversarial round: failure and security/data plus callers,
requested gpt-6-astra high; resolved model and usage not exposed. No concrete
finding. Failure reviewer independently checked sixteen exact Core roundtrips,
eight player-only reads and untouched secondary returns; forty-four individual
flag/type/restriction/getter/guard/family/second-pass changes rejected whole
sections. Three refused exports retained the previous capture. An absent
restriction API preserved stable public false flags. Security/data review ran
compiled and real MCP tests plus ten fresh coverage/registry/unavailable-versus-
false/private-value probes. Both reviews confirmed owner isolation, strict
projection and docs withholding current state, continuity, resurrection rights,
native compatibility and policy approval. No game or deployment probes occurred.

API reference checked from mirrored Forever UnitDocumentation on 10 November,
linked in ADR-079. Getter declarations are source evidence, not authorisation
or installed-client qualification.

Advisory shipping judgement: Jev 1.14.0 selected ready with low confidence 0.28
(ready probability0.52, insufficient_evidence0.38, fix0.10), request
367f38b7490b1468fafd3df04b37d9e5d0b085b97df71f701f7c98cdbb7a2484;
707 input and 44 output tokens, 0.289 seconds. Author accepted the bounded
source-only change based on direct tests and both independent reviews. This
does not resolve native/API/policy uncertainty or authorise deployment/testing;
all parked gates and existing gameplay-action exclusions remain in place.
