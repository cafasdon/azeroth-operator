# Full-stack merge development contract

Source-only Retail executor: `addon/BagMerge.lua`. It is deliberately not loaded by a packaged TOC or exposed through player Apply yet.

The operation identifies both carried stacks by location, GUID, item ID, exact link and count. Locations and GUIDs must differ; item ID and link must match. Both stacks must be unlocked, the cursor empty and the player outside combat, casting and channelling. Character and client identity must match.

Native `C_Item.GetItemMaxStackSize(ItemLocation)` metadata must be present, equal for both stacks and stable across reads. The complete source quantity must fit. Destination bag compatibility and exact preimages are checked before pickup. Matching descriptions do not prove hidden native stack compatibility.

Execution performs one source pickup and one target placement. Rejected placement, exceptions and delayed observations never trigger retries or cursor clearing. Completion requires an empty source and cursor plus the original target GUID, link and item ID with exactly the combined quantity. Target GUID survival is an unqualified assumption; any other result remains unknown.

Offline test `tests/bag_merge.rs` covers quantity conservation, capacity overflow/missing/changing metadata, changed links, locked stacks, duplicate preimages, rejected placement and later read-only completion. Native functions are mocked. This does not qualify native protection, timing or stack restrictions.

## Branch and policy status

The separate Retail branch `feat/bag-merge-transport` contains the
compiler, wire decoder and player review/receipt implementation at
`72e9ab9`. Its branch documentation records offline transport evidence.
The current source retains the executor and its fixtures. The branch
transport commit is outside the current history, and the packaged addon
manifests omit the merge executor and decoder.

The [current MCP policy](operation-policy.md#mcp-enforcement-7-november-2026)
blocks the legacy bag preparation route used by that branch. Forever has
no qualified merge adapter. Native compatibility and placement remain
unverified.

On the current source baseline `fae8fa2`, the offline bag merge executor
fixture passed. The Rust MCP stdio test separately confirmed that the
tool catalogue omits bag preparation and a direct call is rejected.
This verifies the separation between source-only execution fixtures and
the current MCP surface.

`find_snapshot_item` with `bag_stack_groups: true` groups repeated
nonblank, byte-identical links for the requested base item in captured
carried bags. It reports quantities, numeric bag/slot locations and a
separate count for missing links. It shows up to 20 groups and 10
locations per group, with full counts and truncation flags. The grouping
uses the whole saved bag observation independently of item-search paging.
Absent bag data returns null. Links and GUIDs are omitted. The report
leaves native stack compatibility and capacity unverified.

With bag stack groups enabled, `assumed_max_stack_size` adds a hypothetical
minimum stack count and maximum slots freed for each displayed group.
The estimate uses the full group quantity, including undisplayed slots,
and assumes the items can be redistributed freely up to that capacity.
Any recorded stack above the supplied capacity leaves the estimate unknown.
The estimate does not verify capacity, placement, or the feasibility of
any specific merge.
The `capacity_projection_summary` counts estimated and inconsistent
groups across the full observation, including groups beyond the display
limit. Its slot savings sum only groups that fit the assumption.
The same projection counts the maximum number of disjoint pairs whose
combined counts fit the assumption. It counts each saved stack at most
once and sums eligible groups across the whole capture. This describes
one round of whole-stack pairing, with native eligibility unverified.

Review evidence: Jev1.14.0 advisory returned insufficient_evidence (probability0.89, confidence0.83), request b3ad4cbdedd82530f21d67d1c5fc58019053ecc850b9c01c6aa9c13a03ccb37f, 767 input/116 output tokens,0.321s. This is not a review pass or native verification. Required adversarial-review instruction file was unavailable at its configured path. Direct source review confirms two bounded writes and observation-only completion; native assumptions remain explicitly unqualified.
