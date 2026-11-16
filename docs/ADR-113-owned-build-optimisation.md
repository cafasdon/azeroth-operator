# ADR 113: saved ownership in joint build experiments

Date: 16 November 2026

The [owner source](ADR-112-owned-build-source.md) supplies the selected saved
capture to `tools.owned_build_optimizer`. Its configured reader reviews baseline
equipment and resolved physical variants through the existing offline Rust MCP.
Only equipment, bags and a character bank present in that same capture contribute.
Shared banks, mail, historical banks and other characters remain separate.
Missing counts and unresolved modifiers never become candidate copies.

A caller supplies a bounded native raid/encounter model, not inventory or owner
context. Captured equipment replaces model equipment. Existing guards check
captured level, class, race, professions, talents, pet and quiver. The full queried
talent allocation must match every pinned definition without staged changes,
unknown definitions or rank disagreements. This verifies a saved reference;
native legality and model accuracy remain unverified.

Default slot domains contain captured equipment and resolved owned variants
that fit the reference slot. Explicit domains still pass physical-copy, modifier,
class, faction, profession, unique-item and weapon constraints. The pinned talent
generator checks optional rank domains. Each pool retains at most 128 accepted
candidates after at most 4096 scans. Counts and truncation remain visible.
Their Cartesian product uses at most 64 coordinates per native call.

Preparation retains complete manifests only in process memory: eight cases,
4 MiB combined. Reports contain hashes and counts, not native profiles.
Evaluation needs the remembered search identity and identical selector mode.
It selects the source again and rejects changed capture, account, client,
configured source path or default exported character before native execution.
Process restart loses prepared cases; prepare again after restart.

Unsupported levels, absent models, missing sections, incomplete talent coverage
and unresolved equipment produce bound `plan_only` reports without scores.
The native model remains level-60 DPS only. No public hunter fixture substitutes
for a missing personal model. Live identity, current ownership, complete
inventory, native legality, model accuracy and global-best flags remain false.

```powershell
./.venv/Scripts/python.exe -B -m tools.owned_build_optimizer prepare --source ABS_OWNER_CONFIG --catalogue ABS_CATALOGUE --probe ABS_PROBE --engine ABS_ENGINE --input ABS_REQUEST
```

Configuration and request files can contain private character data. Keep them
local and out of Git. No game action, data download or automatic equip exists.
