# Active-pet training points — 8 November 2026

Scope: source and offline verification only. No installation, deployment,
game interaction or native capture.

The public [Forever pet API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/PetInfoDocumentation.lua)
defines `C_PetInfo.GetPetTrainingPoints()` returning total then used points.
The pet-care collector now reads this pair independently of trainer-window
visibility. Both values must be readable signed 32-bit integers. Secret
screening precedes arithmetic/type operations. Missing APIs, errors, partial,
restricted or malformed pairs leave `training_points` absent; zero is a
known value. Both counters must agree across the two existing metadata
reads, otherwise Core marks only pet care unavailable and retains other
valid sections.

The optional wire field is `pet_care.training_points: {"total":25,"used":10}`.
Both keys are required, unknown keys and null are rejected, and the object
is forbidden when the pet is absent. Rust and Python validators agree.
Signed raw observations are preserved; no total/used ordering is invented.
Existing exports remain valid. Reader updates are required for new fields.

`character_summary.active_pet_observation` adds the raw object and
`training_points_balance`, computed as signed 64-bit total minus used.
Deficits remain negative, never clamped to zero; opposite signed 32-bit
extremes remain exact. Unknown counters produce neither field. Existing
`pet_care_change.before` and `.after` include the raw object when available,
without a points-gained/spent delta because captures may describe different
pets. No training, pet naming/GUIDs, stable selection or other actions are
introduced. A point balance does not establish trained skills, lesson
eligibility or simulator training compatibility. Existing trainer reviews
still use their own captured lesson/point observations; no cross-pet merge
is introduced. The simulation projection remains limited to presence,
family ID and level.

Author verification:

```powershell
cargo test --locked --offline --test active_pet_details --test addon
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_pet_training_points test_active_pet_details test_simulator_pet_level
```

Three Rust export/import tests and seven focused Python tests passed.
Coverage includes stable/changed/secret/partial/error counters, zero and
negative raw values, preservation of other sections, actual MCP inspection,
summary and comparison, signed extrema, absent-pet and malformed nested
fields rejected independently by both readers, unchanged legacy metadata
and simulator projection, and existing Hunter-pet level guards.

Development package, not installed:
`local-data/pet-training-qualification/AzerothOperator-0.0.0-pet-training-test-forever-live.zip`.
SHA-256: `5a150ce8ec964bfd5569325844e498637a203770a257f99fe80c48cfecdc1be0`.

Independent callers and failure/security reviews ran on `gpt-6-astra`, high
effort, with no concrete finding. Callers verified actual hex SavedVariables
CLI import at signed endpoints, six independently constructed bad/family
imports rejected by both validators, and the real MCP simulator projection
remaining exactly three fields. Failure review rejected 14 malformed stored
payloads through summary/comparison without echoing a private sentinel, then
confirmed a valid summary still worked. Both re-ran the relevant new tests.
Actual game timing and native secret-value objects remain unqualified.

Jev advisory scope assessment supported offline feature completion while
leaving broad/native completion open: `jev-1.14.0`, request
`a221ae9abe81d1e202be6a4710ca1e470d4571c2d15781b9158662b481f12c34`,
645 input / 51 output tokens, 0.277 seconds. Its uncalibrated probability for
scoped completion was 0.83; actual tests and caller probes provide evidence.
