# Combined bag-space estimates for grouped quest rewards

Date: 10 November 2026. Status: source/offline only.

`inspect_snapshot.quest_turn_in_review` accepts optional
`include_reward_bag_space:true` when `include_reward_review:true`. Both default
false. The existing section, owner-store, reference binding and filtering guards
remain in place. The request now rejects array-form option structs and redacts
deserialisation errors through an object-only adapter. Omission/null retains
existing inspection behaviour.

Each reward review can include `fixed_reward_bag_space`. The unique pre-page
quest union supplies one combined estimate; a quest with several finishers is
counted once. Each displayed giver group's estimate is a separate alternative
view. The existing `group_reward_views_are_additive:false` also applies to space.
No quest choices, quest-item consumption, reward ordering or delivery is modelled.

Aggregate known fixed rewards by base item ID before the existing incoming-item
bag-space helper runs. It uses unanimous stack limits from captured carried
items and captured general free slots. Specialised bag slots do not count as
general capacity. Bank metadata does not supply carried stack limits. Missing or
conflicting limits and unobserved capacity remain unknown. The separate merge
scenario can use hypothetical carried headroom, but neither native stacking nor
placement is verified. Rewards of different variants may not stack in the game.

Known-subset estimates remain available when some quests have unobserved fixed
reward lists. The `all_fixed_rewards_*` estimates and fit fields stay null unless
every included quest's fixed list is observed. A known empty fixed reward set
differs from unknown rewards. Choice rewards remain excluded even when observed.
No pending reward becomes owned stock or spendable funds.

All grouping filters apply before the unique union, which stays independent of
result offset/limit. The existing 256 KiB reward-output guard still rejects large
complete views rather than truncating them; an explicit quest filter can narrow
the requested scope. No new capture, external source, action, route or deployment
is added. Live testing remains deferred.
