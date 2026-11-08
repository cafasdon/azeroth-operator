# ADR 011: Retain numeric equipment material separately from simulation jobs

Date: 9 November 2026. Status: source/offline implementation.

Gear plan journals initially saved identities and coordinate outcomes but not
the equipment behind each hash. Those hashes alone cannot reconstruct future
simulator inputs. The collection controller now offers explicit opt-in retention
of the canonical equipment specification for every resolved review, including
the reused baseline. Defaults preserve hash-only plan rows.

Only the current producer's numeric shape is accepted: an `items` array with 17
positional entries, each empty or exactly `id`, `enchant` and `randomSuffix`.
IDs are positive signed-32-bit values, enchants nonnegative signed-32-bit values,
and suffixes signed-32-bit values. Booleans, unknown keys and partial records
reject. Canonical JSON SHA256 must equal the existing gear hash. All draft rows
and windows in one aggregation/journal must agree on retention. No SQL schema
migration is needed; older absent material remains explicitly unavailable.

The retention flag participates in the controller input binding. Changing it
requires a new journal; no old unbound progress is adopted. Existing payload,
draft/time, SQLite value limits and transactional rejection remain in effect.
Numeric inputs are private owned-gear data and stay in the caller-chosen local
journal. Existing platform privacy boundaries, including the unqualified Windows
folder ACL assumption for journals, still apply.

Bounded merge output includes retained material alongside globally grouped gear
hashes. Exact-window inspection includes it only when explicitly requested and
returns review/hash/spec fields, without selections. Checksums are consistency
checks, not authentication or protection against malicious re-signing.

Retained equipment is not a saved equipment review or qualified executable job.
It omits character-level/class/race/profession/pet observations, native conflict
annotations, templates/APLs, encounter models and engine identity. Future
simulation integration must preserve those independent guards; absence must not
be replaced with fabricated level-60 or other qualifying metadata. This step
does not execute a simulator or establish legal/full owned best-in-slot.
