# ADR 030: resolve ranked gear references into saved loadouts

Date: 9 November 2026. Status: source/offline; no deployment.

Saved-journal ranking exposes gear hashes and source window/review references.
Users also need the actual items and source locations to interpret a result.

Join only the bounded top-ranked entries (at most 20) to checked saved journal
rows in a read-only transaction. Require exact snapshot, search scope and
collection binding, canonical window offset, review index and winning gear hash.
Validate retained equipment and strict source-selection shapes; output only
numeric equipment, source selections, cleared slots and snapshot/report hash.
Close the connection on every path and retain the existing whole-journal
unchanged check after the join. No additional simulation or native action is
introduced. Empty rankings need no join.

Do not promote a saved clear-slot annotation merely because its bytes hash.
Accept only an empty list or the integer offhand slot 17; a clear requires an
explicit mainhand-only selection and empty retained offhand. Preparation must
also match the annotation to the actual MCP review's cleared-slot result.
Missing annotations mean no clear for legacy non-clearing plans; a real clear
with omitted metadata is rejected. The changed preparer fingerprint requires
regenerating previously prepared request artifacts.

Attach `saved_loadout` to each existing ranked entry so standalone journal and
unified all-owned evaluation share the result, including warm checkpoint runs.
The basis is a validated historical plan, not current inventory. Explicitly
keep current-location and native-equip-eligibility verification false. Do not
turn a point-estimate winner into full owned BiS or an executable equip plan.
