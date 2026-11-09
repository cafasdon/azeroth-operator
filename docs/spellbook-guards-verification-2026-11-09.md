# Spellbook namespace/getter guards — 2026-11-09

Baseline ae35fb6. Previous goal turn made progress by landing typed loot capture;
it also exposed a pre-existing spellbook restricted-enum access. This unit fixes
that concrete cause and closely related getter lookups in the same collector.
No game interaction, installation, deployment, schema expansion or new action.
The broader MCP coverage/native/owned-BiS objective remains incomplete and active.

## Observed fault and correction

The spellbook collector read Enum.SpellBookSpellBank.Player before validating the
Enum namespace, its nested table or C_SpellBook availability. Required book API
getters were also invoked without public/function validation. Optional learned
level/usability namespaces were looked up outside protected reads, so inaccessible
optional APIs could suppress an otherwise readable base spell row.

Validate public required namespaces and nested enums before property access.
`bookGetter` checks the current book namespace identity and public function before
each required call/recheck. `spellGetter` keeps optional namespace/function access
inside protected level/usability/resource reads. Unknown optional data remains
omitted; base spell IDs and other independently readable metadata remain intact.
Required failures still become Core's fixed `api_unavailable_or_failed` status.
No casting, actions, event hooks, UI opening or timer access is introduced.

## Red/green author evidence

Environment: Lua 5.1 via mlua, ordinary Core manual slash export and Rust import;
synthetic secret sentinel traps count property/call/comparison attempts. No live
client. Source-only tests are direct commands, not pipeline/full-suite replays.

- Removed the typed-loot fixture's `AO.collectors.spellbook=nil` exclusion.
  `cargo test --locked --offline --test spellbook_guards --test loot_slot_types`
  failed first in typed-loot: 2/4 failed with secretInspections=1 at Enum=SECRET.
- Separate `cargo test --locked --offline --test spellbook_guards` reproduced all
  three failures: required enum access, optional C_Spell restricted access and
  whole manual capture's restricted-read counter.
- Applied the focused collector correction, without widening accepted inputs.
  `cargo test --locked --offline --test spellbook_guards --test loot_slot_types --test spell_resources`:
  3 new guards passed (.01s), 4 typed-loot passed (.48s), 4 existing spell-resource
  tests passed (.10s); compilation 1.94s. These include existing Retail behaviour,
  Forever resource isolation, combat and forbidden action/timer guards.
- Extended the new assertions for 18 required-surface variants (including a
  namespace changed after count) and seven optional-surface variants. Missing
  whole spell namespace omits all optional groups; one unavailable optional
  getter omits only its group while retaining independently readable fields.
  Focused guard tests passed again: 3 tests, .02s, compilation 1.98s.

The full typed-loot manual capture fixture now includes spellbook again and
passes its existing zero restricted-inspection assertion for those synthetic
cases. This does not establish every collector's behaviour under every secret
surface or native API state.

## Uninstalled development package

`cargo build --locked --offline --example verify_addon` passed, 2.03s.
Then:

```text
python -m tools.package_addon --family forever --variant live --interface 16001 --version 0.49.0-dev --output dist/forever-spellbook-guards-0.49.0-dev
python -m tools.verify_addon_packages --directory dist/forever-spellbook-guards-0.49.0-dev --verifier target/debug/examples/verify_addon.exe
```

Exact four-file manifest/source checks, packaged Lua 5.1 roundtrip, Forever
capture fixture and changed-interface refusal passed. live_qualified=false.
Archive SHA-256 efba3ebd6716c4327077e9769b3b8a3117a4687bcdd70782a15f44ad47eeb700.
Collectors SHA cd3d5c9318129aca59fd4cd18656308089450c43ee017b75e0b346763cf0dbcf;
Core unchanged 28f8eded6884753ce25ff2aa4ee08db3e803424cd2df093faf15d27eb401b03f.
The package is uninstalled. Rust MCP/readers and data schema are unchanged.

## Advisory and independent review

Plan Jev 1.14.0 request
06a9f7f85c9b73a50abcf5bff1283276af01165acc02dbeb16ce2dca226b842b:
proceed .94 / revise .05 / insufficient .01; 544 input, 47 output, .349s.
Advisory only. Final independent results/disposition follow below.

## Independent reviews and disposition

Two independent read-only gpt-6-astra high reviewers, Callers and Failure/privacy,
completed one clean round. Both ran all three existing test binaries: 3 guard,
4 resource and 4 typed-loot tests passed. Callers traced Core's failure isolation,
manual slash export, schema-optional metadata and Retail/Forever resource split;
its extracted-package verifier passed exact source rendering and Lua roundtrip.

Failure independently exercised 37 extra synthetic cases: malformed/missing
nested enums, secret required returns/identity fields, namespace replacement,
required lookup errors and optional getter failures. Required failures avoided
restricted values; optional failures retained base rows and omitted affected
metadata, with forbidden-action counters zero. Its verifier passed current source
and extracted package; package collector bytes match after line-ending normalisation.
An initial temporary fixture used the raw adapter, which lacks generated package
identity guards; substituting the packaged adapter corrected that fixture setup.

Reviewers used prebuilt tests/verifier and performed no builds, network, game or
repository writes. Each acknowledged an initial read-only git diff despite the
brief's no-git boundary; no Git mutation occurred. Agent token usage unavailable.
Live/native drift lens omitted because installation and live testing remain deferred.
These synthetic results do not establish actual client secret semantics or
all-collector completeness. The earlier recorded spellbook source guard gap is
fixed by this source unit; old deployed/install state is not inferred.

Final Jev 1.14.0 advisory request
34dbab8e45dc40974b7264549e57e990931bd6133e6850a61f6ddf95dd0307c5:
land_fix .94 / revise .01 / insufficient .05; 588 input, 50 output, .350s.
Author accepts the focused source correction on red/green tests and independent
clean reviews. Final formatting and diff checks passed; broad goal stays active.
