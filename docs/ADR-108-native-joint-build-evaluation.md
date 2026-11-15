# ADR 108: Native joint build evaluation before heuristic search

Date: 15 November 2026. Status: accepted for the bounded offline model experiment.

## Context

ADR 107 identifies joint build coordinates but does not score them. Existing
owned-gear runners do not jointly vary complete talent, rotation, timing and
consumable inputs. A faster heuristic has no trustworthy reference answer yet.
Captured-character checks must not accept a hypothetical talent tree as an
observed one. The existing native CLI also discards unknown protobuf fields.

## Decision

Add a separate explicit model experiment. Resolve content-addressed, disjoint
whole-player-field fragments through the existing case-bound matrix. Preflight
every combination before simulation. Use a pinned protobuf descriptor to reject
unknown fields and aliases, then canonicalise native requests for deduplication.
Use only the qualified positional engine contract and the existing numeric
simulation/cache adapter. Preserve the captured-character workflow unchanged.

The first implementation supports a complete matrix of at most 64 coordinates,
level 60, DPS and one encounter. It returns all coordinate identities and numeric
scores, exact ties and equivalence accounting. Complete matrix coverage is not
native legality, personal ownership, statistical separation or global BiS.
Strict schema acceptance is not proof that every accepted action has an effect.
The output must keep those limits explicit.

The CLI reads one explicitly selected bounded manifest. MCP startup fixes that
manifest and engine. MCP calls cannot substitute a file path, native request or
executable. Optional caching retains only bounded owner-private numeric results.
No game action, network data fetch or live test is added.

## Consequences

This supplies a reproducible exhaustive reference and a real local simulation
caller. It can detect interaction failures in later speed methods. Large-space
search, legal talent generation, multi-encounter scheduling, calibrated racing
and active-character recommendations remain separate dependent work, not implied
by a successful experiment. See optimisation-machine.md for acceptance gates.
