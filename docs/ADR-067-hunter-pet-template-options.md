# ADR-067: Hunter pet template options

Date: 2026-11-09. Status: accepted for offline source integration.

Pinned simulator 9dc32ac8ca0d8d1b76337861d38b93c0f884af11 accepts unknown numeric
PetAttackSpeed values whose missing passive lookup behaves like no passive.
Validate the ten pinned enum names or exact integer values 0 through 9 before
request construction, including inactive pets. Reject duplicate JSON aliases,
booleans, floats, numeric strings and unknown values. Missing/null means zero.

Share pet type, uptime and speed parsing between captured-level preflight and
the ranged observation helper. Prepared merging consequently revalidates all
three template selections. Preserve quoted JSON-number uptime and the existing
captured active-pet level-60 requirement for simulation.

Reports carry template_pet_reference with type, uptime and passive enum name.
Identity, attack speed, training, happiness and native-effect verification stay
false. This is template provenance, not a captured pet match or swing-time claim.
No template changes, raw pet details, game commands or addon changes are added.
The existing adapter fingerprint binds the inline reference and parser.

The pinned-source test checks all four HunterOptions enum groups. It does not
qualify other Hunter fields, native behaviour or the full simulator model.
