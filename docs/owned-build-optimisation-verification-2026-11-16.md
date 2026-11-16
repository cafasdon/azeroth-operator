# Owned build optimisation verification, 16 November 2026

Twelve focused tests passed in 31.541 seconds:

```powershell
./.venv/Scripts/python.exe -B -m unittest tests.test_owned_build_optimizer -q
```

The tests use the real saved-export reader, private account index, pinned
catalogue/equipment probe and native positional engine. The public hunter model
has an explicitly unenchanted wrist: the upstream preset's wrist enchant does
not resolve through the captured-item catalogue projection. Other observed
modifiers stay exact. No private character fixture enters Git.

An actual saved level-60 fixture supplies 128 accepted gear builds and two talent
allocations. The resulting 256-coordinate owned matrix runs a native window at
offset 127, limit two, with positive scores. The previous independent 256-case
native oracle remains in the [joint window record](benchmarks/joint-build-windows-2026-11-15.json).
This test establishes saved binding and execution, not model accuracy or BiS.

Other direct cases cover default gear domains, unowned items/modifiers, shared
bank exclusion, changed active export, explicit-character stability, incomplete
talents, unresolved equipment, unsupported levels, model immutability, unchanged
private files, numeric native enums and a complete empty talent allocation.

The actual owner's local saved capture also passed both default and explicit
selection. Its unsupported level remained unchanged, its missing bank remained
unknown and native simulation did not run. Source hashes stayed unchanged.
Only this boolean acceptance summary is public; identities, items, capture IDs,
account keys and model profiles remain local.

Two independent reviewers requested as Astra/high covered callers/failure and security/data. The
caller lens found numeric protobuf enums rejected by the adapter. Canonical
native names now retain explicitly supplied proto3 defaults. Its scoped recheck
then found an empty talent string omitted by the second canonicalisation; the
same explicit-default preservation closes that path. Final scoped output:

```text
numeric race/class/type/rotation: ready same id: True durationVariation: 0
explicit zero ranks --: ready
empty talent string: ready
empty talent evaluation: 1 positive DPS: True
```

The security lens also tried physical-copy duplication, invented modifiers,
identity/talent overrides, oversized models, changed sources before a missing
engine, account/shared-bank stock and cache case/byte limits. It found no medium
or high fault. A ring with one current-bank copy stayed rejected despite 100
shared/foreign copies; two current-bank copies passed. Saved hashes stayed equal.

Review used three rounds: one full and two scoped caller rechecks. Two medium
compatibility faults were fixed. No medium/high findings or nontrivial lows remain.
The fixed owner authority boundary and generic CLI error route stayed intact.
Native source/stdio execution is distinct from live character qualification.

Typed advisory evidence, all `jev-1.14.0`:

| Stage | Request SHA-256 | Tokens in/out | Seconds | Disposition |
| --- | --- | --- | --- | --- |
| Numeric enums | `dac184b31555fde66a7bf12e50d4b1c7714adb58707e9ec1ee42720f6069b1f7` | 452/48 | 0.327 | fix medium |
| Empty talent default | `49d7b4c0488cab58d105cdd69beff634d6880911e48bf7dcd58597946fa3d8d6` | 470/48 | 0.319 | fix medium |
| Core release | `80ad7c6f824c187d547f794318b05c91ea1ab49fe29ecdf2ab47f11e393f8f6b` | 594/45 | 0.300 | ship |

Agent requests used `gpt-6-astra/high`; actual resolved model and usage counters
were not exposed. Live tests remain deferred.
