# ADR 028: explicit opt-in positional equipment engine contract

Date: 9 November 2026. Status: isolated source/offline engine extension;
MCP/adapter integration pending.

Historical status above records the isolated engine decision. Subsequent
integration is specified in [ADR 029](ADR-029-positional-equipment-integration.md)
and its [offline evidence](positional-integration-verification-2026-11-09.md).

The pinned engine's `ProtoToEquipment` uses automatic placement, so an item in
the second ring/trinket slot with the first empty is relocated. Existing MCP
guards correctly withhold those simulator drafts. Removing the guard or silently
moving the requested item would not model the exact desired layout.

Add an isolated additive protobuf contract: EquipmentSpec field 2,
`preserve_slots` (`preserveSlots` in JSON). Default false delegates to the exact
original conversion expression. True requires exactly 17 explicit slot objects,
constructs each item in that slot and preserves holes. Use existing item-fit and
enchant applicability rules. Reject nonempty modifiers on empty slots, any
requested ID/modifier not retained by the underlying constructor, invalid item
slot/enchant fit and two-hand/off-hand conflicts. Native/class/level/unique-set
eligibility and complete combat-model qualification are separate requirements.

Expose a read-only `azeroth-equipment-contract` command and a separate numeric
equipment probe. The incumbent CLI uses `DiscardUnknown: true`; it silently
ignores the new flag. A future integration must verify trusted engine hash and
explicit capability before requesting this layout. Neither an arbitrary flag
nor a caller-provided capability label proves an engine supports it. Current
MCP/adapter safeguards remain unchanged until that integration is qualified.

Provide a download-free Windows amd64 builder from exact local source/SDK
archives, fixed protobuf tools and trusted dependency/cache inputs. Retain the
upstream MIT notice. Work only in a fresh ordinary directory, bound ZIP size/
containment, require exact patch anchors, seal Go's persisted/workspace/private/
proxy/VCS/telemetry/target/flag settings and disable toolchain fetching. Child
commands have a shared deadline and owned process-tree cleanup on timeout or
interruption. Partial work is retained for inspection; existing outputs are not
overwritten. Cached modules/build results are explicit trusted build inputs.

The manifest records input/source/tool/output hashes and leaves qualification
flags false: a successful compiler is not model proof. Separate tests compare
numeric slot/modifier fidelity, malformed rejection, capability and canonical
simulation metrics against the incumbent. Keep engine identity separate from
numeric equality. Native game, ownership, all-class/encounter accuracy and full
owned BiS remain unverified. No addon/client install, deployment, game action or
automatic engine replacement is introduced.
