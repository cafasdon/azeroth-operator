# Private pilot — 14 November 2026

## Scope and evidence

The owner authorised native testing on the logged-in Forever character. This
pass called every one of the 12 installed hosted tools, across 14 requests.
Successful responses were checked against the locally imported real export,
the earlier capture, and native screenshots. This is a bounded smoke test,
not qualification of every optional argument or every feature branch.

Opening Leatherworking, exporting with the wallet option, and reloading added
five sections to the real capture: profession spells, filtered recipe view,
bulk materials, selected recipe and wallet. The strict installed runtime
accepted the export, and the normal restricted uploader sent it. Authenticated
HTTPS MCP calls read the new capture. There were 40 observed sections.

The first manual uploader invocation used a relative config path and failed
the documented absolute-path check. The corrected absolute-path invocation
succeeded. This was an operator invocation error, not a product failure.

Raw exports, character identifiers, item GUIDs, wallet amounts, tokens, native
screenshots and detailed response bodies remain in ignored local storage.
The committed [redacted result](private-pilot-results-2026-11-14.json) contains
only aggregate outcomes and public item references.

## Checks

| Tool or path | Result and limit |
| --- | --- |
| Capabilities | All 12 default tools reported; analysis mode retained. |
| Snapshot inspection and listing | Exact uploaded capture appeared for the explicit saved character. |
| Explicit character resolution | Selected the exact latest capture with bags and equipment. |
| Resolution without a target | Returned `active_identity_unavailable`, no selection and no active-character claim. |
| Summary | Matched 13 equipment slots, 17 bag stacks and 40 observed sections. |
| Snapshot comparison | No level or earned-XP change; recorded rested XP increased by 100. Profession ranks stayed unchanged. |
| Item lookup | Located the equipped gloves in slot 10 with quantity one; unavailable banks remained null. |
| Equipment review | Re-selected the same saved gloves as a draft, with zero stat deltas. A deliberately wrong expected item ID was rejected. No item was equipped. |
| Talent export | Returned `unavailable` / `client_not_supported` for Forever; no invented import string. |
| Recipe review | Camp Tent matched the native display: five Light Leather required, zero carried, five short. Missing banks remained unknown. |
| Recipe restock | One requested tent produced a one-craft draft with the same five-leather shortfall. No purchase or craft occurred. |
| Bulk recipe search | Returned zero rows from the empty captured profession-spell/material lists despite a populated filtered recipe view. This path is not qualified. |

Native profession ranks matched the capture: Leatherworking 50/75, Skinning
126/150, First Aid 60/75 and Cooking 31/75. No movement, combat, trade,
purchase, social message, quest acceptance or inventory mutation was part of
this pass. UI panels and local exports changed.

## Open pilot work, in order

1. **Bulk recipe capture discrepancy.** The ordinary local Leatherworking UI
   showed known recipes and the filtered view captured them. The separate
   profession-spell API path captured an empty list, so bulk material search
   returned no recipes. Reproduce the native API inputs and semantics before
   changing the collector; do not infer known recipes from the public database
   or silently relabel a filtered view as complete knowledge. Acceptance:
   known native recipes reach bulk review, or the unsupported source produces
   an explicit inconclusive result. Preserve source/secret/context guards.
2. **Current-character selection.** Explicit saved identity works. Automatic
   current-character identity does not exist in the installed offline reader.
   A newest capture alone cannot prove which character is currently in play.
   Acceptance needs an owner-scoped current-session signal, expiry, logout and
   character-switch tests, and explicit-target precedence.
3. **Forever talent and simulator qualification.** The current talent-import
   endpoint does not support this client. No best-in-slot or full optimisation
   claim follows from a successful equipment draft. Acceptance needs a qualified
   level/class/talent/gear mapping and an independent reference result.
4. **Remaining native coverage.** Banks, mail, merchants and player resources
   still need separate native tests. This pass did not establish that closed
   panels or unavailable APIs can supply these inputs.

No runtime source changed in this pass. The four items above remain open;
successful transport and safe rejection are not evidence that these gaps are
complete. The prose-only evidence update requires no additional code review.
