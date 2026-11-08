# Hunter pet simulation level — 8 November 2026

Source and offline qualification only. No installation, deployment, game
interaction or native capture.

The pinned engine sets all core pets to `CharacterLevel` in
[`core.NewPet`](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/pet.go),
which is 60. Hunter [`NewHunterPet`](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/hunter/pets.go)
constructs a pet only for a recognized family and positive pet uptime.
The public [`HunterOptions` proto](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/hunter.proto)
has pet type and uptime, but no configurable pet level. A level-60 player
with a lower-level pet must not be simulated as having a level-60 pet.

Equipment reviews now project only `present`, `family_id` and `level` from
the validated saved `pet_care` section as `captured_active_pet`; unavailable
section is null. No pet name, GUID, loyalty or food records are added to
the projection. Direct simulation and ranking require a saved present pet
with exact integer level 60 whenever the Hunter template models a pet.
Missing/absent/unknown or other levels stop simulation with a fixed diagnostic.
Plan-only ranking retains the draft and reports unsupported/unverified level.
Disabled pets, zero uptime and other player specialisations do not require
this Hunter-pet level observation.

Both protobuf JSON spellings of class options, pet type and uptime are
recognized; duplicate aliases are rejected. Null behaves like proto default.
Pet types use the pinned enum; uptime must be a finite number from zero to
one, excluding booleans. Quoted numeric/exponent double values are accepted
according to [ProtoJSON](https://protobuf.dev/programming-guides/json/), with
conversion used only for checking/reporting; original template values remain
unchanged. Raw simulator
requests used for public synthetic qualification remain separate from
character-bound equipment-review construction.

Reports appear under `template_identity.hunter_pet` for ranking and
`hunter_pet_assumptions` per review for the direct adapter. A matching level
does not establish family, trained skills, happiness, pet talents or attack
speed compatibility; those remain unverified. No native family-ID to
simulator-pet-enum mapping is asserted. This does not verify Warlock or
other companion configurations. Old review exports need an updated-reader
re-export; an existing snapshot containing pet level 60 suffices without
another game capture. Missing native metadata still needs later capture.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_simulator_pet_level test_active_pet_details test_simulator_level test_simulator_character_identity test_simulator_professions test_encounter_sensitivity
```

Build passed; 22 tests passed. Tests cover actual MCP projection of known,
missing and absent pets; canonical/snake-case options; missing/lower/invalid
pet levels; disabled models; invalid enums/uptimes including huge integers,
NaN and infinities; duplicate aliases; input immutability; an actual direct
module CLI with a mismatched second review and nonexistent engine; and
ranking/plan-only behavior with engine entry points asserted untouched.

Independent callers and failure/security reviews ran on `gpt-6-astra`, high
effort. The first round found one compatibility defect: numeric strings
such as `"1"`, `"1e0"` and disabled `"0"` were incorrectly rejected for the
double uptime field. The author verified the official ProtoJSON contract,
fixed only local decision-value parsing and added regressions. All five pet
level tests then passed. The final review round found no further concrete
flaw: 16 quoted-uptime direct/ranker cases agreed, stopped unsupported levels
before the engine and preserved inputs. Failure review checked pinned
protobuf-go v1.36.10's numeric parser and malformed/nonfinite strings.
The initial caller review also passed five raw-request shortlist qualification
tests, actual MCP projection, bad additional-draft fixed/bulk/finalist checks
and all three module help entry points. No live testing was performed.

Jev advisory triage recommended fixing the reproduced compatibility issue:
`jev-1.14.0`, request
`bbd15715a52c7c10504c3b2575d69a0acc8d440ebd776647e3b96db0827d9f82`,
529 input / 46 output tokens, 0.284 seconds. Source and independent tests,
not the advisory probability, establish the correction.
