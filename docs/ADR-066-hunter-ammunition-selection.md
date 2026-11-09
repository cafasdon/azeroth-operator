# ADR-066: Pinned Hunter ammunition selection boundary

Date: 2026-11-09. Status: accepted for offline source integration.

## Problem and decision

Pinned9dc32ac NewHunter selects an arrow reference by Ammo.String() and adds its
DPS to BonusRangedDps. Unknown numeric protobuf enum values survive decoding but
find no reference, producing a zero-valued effect. Review preflight previously
accepted such values, silently changing model meaning.

Validate Hunter ammo selection inside the existing ranged/quiver observation
helper used by request_from and prepared merging. The inline fingerprinted map
contains AmmoNone plus the nine exact names/IDs/DPS values in pinned
proto/hunter.proto and sim/hunter/ammo_auto_gen.go. Accept enum names or integer
IDs only; reject booleans, float/string-numeric forms and unknown values with
fixed errors. Existing classOptions/class_options alias checks remain; missing
or null ammo uses the existing protobuf-option none default. No valid selection
is normalised or changed in the resulting request.

## Evidence output

The historically named hunter_quiver_observation report adds nested
template_ammo_reference with basis pinned_template_selection_only, item_id,
bonus_ranged_dps, and false identity_verified/availability_verified/
native_effect_verified flags. The value is a pinned ranged-weapon DPS parameter,
not expected player DPS improvement. AmmoNone records reference ID0/DPS0, never
an observed empty ammunition slot. Non-Hunter not_applicable reports stay as
before. Existing top-level ammo_identity_verified remains false.

Existing preparation/merge validation recomputes the report; saved-window and
final encounter contexts carry it without new request fields. Unsupported enums
reject before engine/cache access in these review/preparation workflows, also
with fresh self-consistent request hashes. The lower-level raw-request/engine
APIs are not newly qualified by this guard.

## Boundaries

No equipped-ammo capture, stock/consumption proof, native-effect qualification,
quiver/ammo optimisation, automatic template correction or game action. Captured
bag/item ID coincidence does not establish selected ammunition or availability.
No new module outside existing adapter fingerprints, snapshot schema, addon or
deployment. Full owned-gear/native/model coverage remains open.
[Evidence](hunter-ammunition-selection-verification-2026-11-09.md).
