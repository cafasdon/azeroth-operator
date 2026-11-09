# ADR 022: starting-stock preparation for printed recipe chains

Date: 9 November 2026. Status: source/offline implementation.

Chain expansion can allocate observed bank stock and conditional intermediate
outputs, but the caller also needs to know which items must exist before
starting its printed order and which observed units might need withdrawing.
Add opt-in `reagent_chain_stock_plan` under the existing recipe review, requiring
chain mode. A complete chain receives nested `stock_plan`; a blocked chain
receives null. Defaults remain unchanged.

Revalidate each exact printed step against the same recipe/inventory captures,
including recipe ID, craft count, output ID, minimum output and fixed reagent
quantities. Permit repeated producer IDs in different steps. Reuse the existing
ordered-batch helpers to compute maximum prefix deficits from zero stock:
consume each step's requirements before adding its reported minimum output.
Those deficits are minimum initial quantities for this printed order under the
stated yield assumptions, not a globally optimal crafting route.

Compare starting needs with saved bags and all observed storage. Existing
bank-restock rules suggest character-bank units first, then account-bank units,
up to known shortfalls. Missing observations remain null. Items fully supplied
by earlier conditional outputs can require zero starting units. Preserve legacy
field names such as `required_before_batch` and `recipe_number`; here they refer
to the whole printed chain and its one-based step positions, including repeats.
The material checkpoint is against all observed storage, not carried bags.

Do not include hypothetical external acquisitions as observed stock. An
affordable merchant quote can therefore coexist with an uncovered preparation
checkpoint. Nonmaterial requirements are reported separately and remain unknown
where captured checks are missing. Current stock, minimum yield, withdrawal
eligibility and craftability remain unverified; no bank transfer or game action
is issued. Native bank accessibility, execution and broad profession planning
remain separate unfinished requirements.
