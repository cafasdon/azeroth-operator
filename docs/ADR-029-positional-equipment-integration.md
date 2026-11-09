# ADR 029: qualify positional equipment at the execution boundary

Date: 9 November 2026. Status: source and offline integration; no deployment.

The incumbent engine silently ignores unknown fields. The positional flag from
ADR 028 therefore cannot by itself authorise exact-slot simulations.

Expose `positional_simulator_equipment`, default false, on the Forever catalogue
equipment review. Apply it consistently to the primary and additional drafts.
Only this mode bypasses paired-slot relocation blockers; all other resolution
and legality checks remain. Resolved specs carry `preserveSlots: true` and the
review identifies `azeroth_equipment_preserve_slots_v1`.

Propagate the mode through the explicit `--positional-equipment` collection and
all-owned evaluation options. Include it in plan scope, retained canonical spec
hashes, cross-window validation and current MCP preparation. Fingerprint the
layout helper with the other software inputs. Reject mixed-mode journal resumes.

Require the qualified Windows amd64 engine SHA-256
`833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`
and its exact bounded capability response. Check hashes before and after the
capability command. Validate before all-owned collection writes, adapter cache
lookup and saved-result checkpoint lookup. A plan-only collector needs no
engine; every execution path still validates it. Reject positional bulk execution
because that path has no exact-slot qualification.

Keep the legacy layout and four-argument Rust catalogue API unchanged by default.
Old collection bindings and prepared artifacts may require recollection after
software fingerprints change; never silently reinterpret their mode. Saved
journals remain intact on rejection.

The limited offline qualification establishes propagation, exact slot retention,
bounded work and numerical compatibility on recorded synthetic/public inputs.
It does not establish complete ownership, all-class combat accuracy, native
eligibility, current-character selection or full best-in-slot optimality. Game
testing, installation and deployment remain deferred.
