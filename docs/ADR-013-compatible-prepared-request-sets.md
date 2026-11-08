# ADR 013: Group compatible prepared windows before execution

Date: 9 November 2026. Status: source/offline implementation.

One prepared window can reuse the same baseline or gear input as another.
A separate bounded aggregator now forms a deduplicated request set before any
engine execution. It reads selected source journal rows in one read-only
transaction, verifies their schema/digests/retained material, and requires each
prepared artifact's source hash, snapshot, scope, counts, ordered local reviews
and numeric equipment to agree with its saved row.

Canonical request hashes must agree with contents. The complete request context
after removing only the simulated player's equipment must match across every
request. This includes the raid/profile/APL, encounter, iterations and seeds.
MCP/template/encounter/preparer/adapter/catalogue fingerprint labels must also
agree. Gear hashes group requests only under that context; repeated keys must
have identical complete requests. Global review IDs use the existing baseline-
first, hash-sorted order, preserving original window/local-review references.

The tool accepts at most 64 artifacts, each at most 4 MiB. Prepared input bytes,
selected source report bytes and final output each have a 16 MiB aggregate limit.
Existing SQLite value and deadline bounds remain. Duplicate/overlapping windows,
partial request lists and incompatible contexts reject atomically with no CLI
partial result or journal writes. Input paths and raw selection payloads are
omitted; output still contains private supplied profile/APL material.

This validates consistency of explicitly supplied artifacts, not authentication
or independent proof that their claimed preparation ran. Hashes and labels are
not signatures. It checks selected rows rather than every journal row, and
reported coordinate coverage remains separate from feasible-set/model coverage.
Partial window sets keep explicit gaps. No engine or executable is accepted,
no MCP/game call is made, and all simulation/game/BiS/feasible flags remain false.

The bounded set is a step towards avoiding redundant simulator work. An engine
identity, execution/result journal, scalable resumable work beyond this bounded
set, global result ranking/uncertainty and full owned BiS remain future work.
