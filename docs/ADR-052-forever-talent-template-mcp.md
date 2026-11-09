# ADR-052: Direct MCP review of Forever simulator talent strings

Date: 2026-11-09. Status: accepted for offline source integration.

Extended by [ADR-055](ADR-055-queried-talent-allocation.md) with a separate,
explicit preparation mode; the comparison mode described here is preserved.

## Problem and decision

ADR-051 connects captured purchased ranks to local simulation commands. The MCP
itself could only return a native Retail import string, leaving this comparison
unreachable through its advertised read-only tools.

Extend only `captured_talent_loadout` with optional `simulator_talent_string`.
Use its own strict request structure so other tools sharing `Summary` do not
silently accept the new option. The snapshot hash remains required and selects
one capture in the installed user's configured owner store. No class override,
newest-character fallback or active-character inference is added.

Example arguments:

```json
{"snapshot_id":"<explicit saved content hash>","simulator_talent_string":"1"}
```

The string uses the pinned simulator's up-to-three digit-tree layout, not a
Blizzard talent import encoding. It must be ASCII digits and hyphens, at most
256 bytes, with at most three parts. Known class tree lengths are bounded by
the actual definition count, including the documented Priest final-tree rule.
Empty/trailing trees mean zero, as in the engine. These syntax checks apply even
when the capture lacks optional talent metadata; bad text is never called valid
merely because observations are missing.

With the option supplied, Forever responses add `simulator_talent_observation`.
The native loadout status remains `unavailable`/`client_not_supported` because this
feature does not produce a native import. Without the option, old response
semantics stay unchanged. Retail continues to return its captured native string;
supplying this Forever-only option on Retail/Classic is rejected.

## Observation contract

The Rust reader embeds the same SHA-256-verified public map as ADR-051. Docker
already copies `clients/` before compilation, so the runtime needs neither a
network lookup nor a readable external map file. Class comes from the saved
stats section. Missing/unmapped class returns `unverified` with reason
`captured_class_unmapped`; missing/legacy talent metadata stays unverified.

For validated modern metadata, numeric output matches the Python adapter's
non-throwing review mode: definition/purchased rank versus template effective
rank, compared/unknown/native-limit-disagreement counts and observed staged flag.
Duplicate observed definition IDs reject the ambiguous request. Active boosted
ranks are not compared to the simulator's base purchased ranks.

Statuses are `mismatch`, `observed_ranks_match`, `staged_changes` or `unverified`.
Known conflicts are data for review; no simulator is invoked by this MCP query.
Unknown staging leaves conflicts conditional and status unverified. Unknown
definitions or native maximum disagreements cannot establish equality. All
allocation_match_verified, native_tree_coverage_verified and
simulation_model_verified flags remain false.

The new nested result never echoes the requested string, native config names,
saved-loadout names or import text. The surrounding response retains the existing
owner-scoped character identity and capture provenance. Errors are fixed text.
No gameplay command, mutation, automatic talent selection or policy expansion.

## Verification boundary

Actual local stdio MCP tests compare all 467 mapped definitions in synthetic
fixtures across all nine classes, plus first/last positions of all three trees,
against the existing Python checker. Fixtures intentionally do not claim legal
point allocations. Partial/staged/unknown and 4096-node cases retain uncertainty;
malformed requests and unsupported families fail without private canary leakage.
Retail and omitted-option behaviour have regression assertions.

Hosted HTTP, deployment, native capture acceptance, full allocation/model truth
and fresh active-character identity remain unqualified. This unit changes no
addon source or installation. See the dated verification record.
