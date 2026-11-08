# Active-pet public details — 8 November 2026

Scope: source and offline verification only. No installation, deployment,
game interaction or native capture.

Forever `pet_care` now accepts optional `family_id` and `level`, each a
positive unsigned 32-bit integer and valid only when `present` is true.
Explicit null is rejected; missing remains unknown. Rust and Python readers
enforce the same contract. Old captures remain valid.

The collector reads only the second return of `UnitCreatureFamily("pet")`
and `UnitLevel("pet")`. The public [Forever unit API documentation](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitDocumentation.lua)
lists the family name/ID tuple and numeric level return. The family name
is discarded, even when restricted. Values are checked for restrictions
before numeric operations. Missing APIs, errors, unknown level sentinels
and malformed/restricted values omit optional fields. Two reads around
the existing care capture must agree, and the pet must still be present;
otherwise Core marks only this section unavailable. No pet GUID, individual
name, cache-load request, unit action or automation is introduced.

`character_summary.active_pet_observation` returns presence and nullable
family/level, or null when no pet-care section was captured. Existing
`pet_care_change.before` and `.after` include known fields without deriving
a level delta. Family/level equality cannot establish that two observations
refer to the same individual pet. This does not qualify pet skills, talents,
stable roster, training eligibility or simulator pet compatibility.

Author verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test active_pet_details --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_active_pet_details test_pet_view_comparison
```

Build passed. Three Rust tests passed, using actual Lua collectors/Core and
the Rust importer: known details, discarded private name, unknown/restricted
values, invalid sentinels, API failures/missing APIs, absent pet and changing
family/level with equipment preserved. Two real MCP integration tests passed:
inspection, summary, comparison observations, missing/absent distinctions,
16 malformed/absent-pet field cases rejected by both readers, and existing
filtered pet-journal comparison behavior.

Development-only package (not installed):
`local-data/active-pet-qualification/AzerothOperator-0.0.0-active-pet-test-forever-live.zip`.
SHA-256: `4a2d455c5007314b49098d7cb59ed0c98078b3223e553af05a27a25c95ac9a9f`.

Independent caller and failure/security reviews ran on `gpt-6-astra`, high
effort, with no concrete finding. Callers re-ran the scoped Rust/addon and
Python MCP tests and passed 12 independently constructed Rust/Python import
cases across all families, optional single fields, maximum u32 and absent-pet
rejection. Failure review re-ran both new tests and passed 35 hand-built
Rust/Python cases, including malformed types, both u32 boundaries and
rejection of injected individual-name/GUID/family-name fields. Native secret
values cannot be reproduced by the fixtures and remain unqualified.

Jev advisory scope assessment supported offline feature completion while
leaving the broad goal and native testing open: `jev-1.14.0`, request
`a57fdd4d9e77c4e967b120a1ccca8de8bf8be1d770a342a0449fb368e6cee3d7`,
590 input / 47 output tokens, 0.306 seconds. Tests and real caller probes
provide the evidence; the advisory does not establish native compatibility.
