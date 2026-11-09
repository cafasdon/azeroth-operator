# ADR 038: cumulative captured charges for repeated chain producers

Date: 9 November 2026. Status: source/offline; no deployment.

Chain expansion deliberately permits repeated producer steps when later branches
need more intermediate items. Materials use a shared ledger, but independent
per-step charge checks could each reuse the full saved charge count. One saved
charge could appear to cover both of two separate one-craft producer steps.

Maintain cumulative requested crafts per recipe ID across completed printed
steps. Use only the original bulk recipe's observed charge count. Expose modeled
charges before/after each step, cumulative deficit and coverage, retaining the
existing per-request check separately. A missing charge count remains unknown,
even when a timer is present. Different recipe IDs use independent accounting;
no shared cooldown groups or elapsed recharge can be inferred from current data.

Reuse the ledger in optional chain stock preparation's ordered nonmaterial
requirements. Add a cumulative check to the first observed shortfall and attach
one accounting row per printed step. Keep ordinary unique-recipe batch/dependency
output unchanged. The internal ledger only receives validated, same-recipe-
capture review rows: at most eight steps, each one to 1,000 crafts.

Charge shortfalls do not block conditional material expansion: draft completion
means the bounded material dependency draft was derived, not that it can be
crafted now. Blocked drafts retain accounting only for completed prefix steps.
No simulation of waiting, native action, storage ownership, minimum-yield,
craftability or policy-compliance guarantee is introduced.
