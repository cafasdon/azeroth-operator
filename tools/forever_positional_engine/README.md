# Isolated positional equipment extension

These consumer-owned Go fragments extend pinned `wowsims/forever`
`9dc32ac8ca0d8d1b76337861d38b93c0f884af11`. Upstream software is MIT-licensed;
the builder retains its complete notice in the source tree and artifacts.
It does not redistribute the source archive or change its licensing.

Build with `python -m tools.build_forever_positional_engine --help`. This is a
Windows amd64 offline developer build, requiring the exact locally cached
source/Go archives and protobuf tools recorded in that module, a trusted module
cache and a fresh work directory. Optional build-cache reuse is explicit.
Missing dependencies fail; no module or SDK download is attempted by the build.
Partial work/logs remain inside the fresh directory on failure. No installation,
MCP configuration change or incumbent binary replacement occurs.

`EquipmentSpec.preserve_slots` / JSON `preserveSlots` defaults false. False uses
the original automatic placement. True requires 17 explicit slot objects and
directly preserves their positions, including paired-slot gaps. Reject invalid
slot fit, empty-slot modifiers, lost modifiers, incompatible enchants and a
two-hand/off-hand conflict. This is basic model construction, not complete
character eligibility, native equip verification or an ownership claim.

The built engine exposes `azeroth-equipment-contract`; the separate numeric
equipment probe reports the resulting IDs/enchants/suffixes without running a
raid. A build manifest records source/SDK archive, protobuf tool, builder/fragment
and output binary hashes, with qualification flags false. Dependency and build
caches remain trusted, unhashed inputs. Independent test evidence is separate.

The incumbent CLI silently discards unknown protobuf fields. Sending the wire
flag alone is unsafe: a future consumer must verify the trusted engine hash and
explicit capability before opting in. Current MCP and Python adapter guards are
unchanged, so this extension is not yet used by owned-gear evaluation.

See [ADR 028](../../docs/ADR-028-positional-equipment-engine.md) and
[offline evidence](../../docs/positional-engine-verification-2026-11-09.md).
