# Reward and budget review for saved turn-in groups

Date: 10 November 2026. Status: source/offline only.

Add `include_reward_review: true` to the existing turn-in review page. Omitted
or false preserves the previous output. Each returned group receives a saved
potential-reward view for its distinct quest IDs. A top-level
`unique_quest_reward_review` uses the union of all matching retained groups
before pagination, so alternate NPC/object finishers cannot multiply rewards.
It stays unchanged across pages, including beyond-end pages. Ungrouped quests
are excluded from that union and retain the existing coverage diagnostics.
`group_reward_views_are_additive: false` makes this boundary explicit.

Reuse the existing saved XP, wallet, fixed-item, currency-cap and renown
projections for each subset. Add captured honor totals and unknown counters.
The same snapshot supplies experience, wallet, currency and reputation context;
missing context stays unknown. Observed zero remains zero. Group/filter options
continue to select potential quests regardless of current state; raw readiness
groups remain unchanged. Existing wallet/XP planning rules still account for
failed or unknown readiness and disabled XP. Their models are independent,
not a combined optimum or executable sequence.

Fixed item/currency rewards are aggregated; item and currency choices are
identified but not applied. Spell rewards are not projected. Potential sums
are not delivered rewards or spendable funds. Renown/currency projections use
only saved balances and caps. Cross-quest exclusivity, native eligibility,
delivery, route quality and complete finisher coverage remain unverified.
No source refresh, provider query, inventory credit or gameplay action occurs.

With reward review enabled, the serialized review `data` is capped at 256 KiB;
oversized requests reject, never silently discard rewards. Narrow the page,
quest IDs or giver filters to obtain smaller views.
If the union alone is too large, paging cannot shrink it; narrow quest/giver
filters instead. This cap is on review data,
not a new claim about the byte size of the surrounding MCP transport wrapper.
Missing source/reference sections keep the unique projection null; known empty
matching sets return known zero totals. Data stays owner-local and outside the
public knowledge index. Installation/deployment and native testing stay deferred.
