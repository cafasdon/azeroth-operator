# ADR 023: historical public recipe craftable counts

Date: 9 November 2026. Status: source/offline implementation.

Material capacity alone cannot reproduce the profession UI's reported available
craft count. Retain an optional `reported_craftable_count` on fixed bulk material
rows, using `C_TradeSkillUI.GetCraftableCount(recipeID)` with no recipe-level
argument, matching the public recipe-list call. Do not infer which storage or
other constraints contribute to that opaque result.

Read twice, protecting both reads. Retain only equal, public integers from zero
through u32 maximum. Missing API, errors, secrets, nil, invalid or changing
values omit the count without discarding material evidence. Only retained fixed
rows are eligible; existing 4,096-row and material budgets remain. At most two
additional count calls per retained fixed row are introduced. Native capture
latency and availability must still be qualified separately.

Both strict snapshot readers enforce the same optional non-null u32 contract.
Exact bulk recipe review exposes the value, recipe capture timestamp, public API
default-level basis and saturated shortfall against requested crafts. An explicit
inventory override changes material accounting, never this observation's source.
Missing counts add no observation. Raw recipe search retains the optional field.

The count does not gate or change material capacity, learning, ordered batches,
chain expansion, quotes, stock preparation, cooldowns or charges. It is historical
evidence, not current craftability or authority to craft. No game action, data
sync, installation or deployment is added. Full progression and native policy
qualification remain separate unfinished work.
