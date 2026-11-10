//! Deterministic grouping of historical provider finisher references only.
use crate::{Family, Snapshot, analysis, quest_references};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GiverKind {
    Npc,
    Object,
}

impl GiverKind {
    fn label(self) -> &'static str {
        match self {
            Self::Npc => "npc",
            Self::Object => "object",
        }
    }
    fn source(self) -> &'static str {
        match self {
            Self::Npc => "npcs",
            Self::Object => "objects",
        }
    }
}

fn default_limit() -> usize {
    20
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(default)]
    pub include_reward_review: bool,
    /// Estimate space for known fixed rewards only; requires include_reward_review.
    #[serde(default)]
    pub include_reward_bag_space: bool,
    pub ready_for_turn_in: Option<bool>,
    pub giver_kind: Option<GiverKind>,
    pub giver_id: Option<NonZeroU32>,
    pub quest_ids: Option<Vec<NonZeroU32>>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

const MAX_REWARD_REVIEW_BYTES: usize = 256 * 1024;

fn bounded_result(result: Value, include_rewards: bool) -> Result<Value> {
    if include_rewards {
        ensure!(
            serde_json::to_vec(&result)?.len() <= MAX_REWARD_REVIEW_BYTES,
            "Quest turn-in reward review exceeds 256 KiB output limit"
        );
    }
    Ok(result)
}

pub fn argument<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<Page>, D::Error> {
    Option::<Value>::deserialize(input)?.map(|value| {
        if !value.is_object() {
            return Err(serde::de::Error::custom("Invalid quest turn-in review options"));
        }
        serde_json::from_value(value)
            .map_err(|_| serde::de::Error::custom("Invalid quest turn-in review options"))
    }).transpose()
}

fn reward_review(snapshot: &Snapshot, ids: &BTreeSet<u32>, include_bag_space: bool) -> Value {
    let quests = snapshot.sections["quests"].as_object().unwrap();
    let subset = ids
        .iter()
        .map(|id| {
            let key = id.to_string();
            let quest = quests[&key].clone();
            (key, quest)
        })
        .collect();
    let mut honor = 0u128;
    let mut unknown_honor = 0;
    let mut currency_choices = Vec::new();
    let mut unknown_currency_choices = 0;
    for id in ids {
        let quest = &quests[&id.to_string()];
        match quest["reward_amounts"]["honor"].as_u64() {
            Some(value) => honor += u128::from(value),
            None => unknown_honor += 1,
        }
        match quest["currency_choices"].as_object() {
            Some(choices) if !choices.is_empty() => currency_choices.push(*id),
            Some(_) => (),
            None => unknown_currency_choices += 1,
        }
    }
    let mut report = json!({"scope":"saved_potential_numeric_and_fixed_item_rewards", "quest_ids":ids,"quest_count":ids.len(),
        "xp":analysis::quest_list_xp_projection(&subset,snapshot.sections.get("experience")),
        "wallet":analysis::quest_list_wallet_projection(&subset,snapshot.sections.get("wallet")),
        "fixed_items":analysis::quest_list_item_rewards(&subset),
        "currencies":analysis::quest_list_currency_projection(&subset,snapshot.sections.get("currency_balances").unwrap_or(&Value::Null)),
        "renown":analysis::quest_list_renown_projection(&subset,snapshot.sections.get("reputation").unwrap_or(&Value::Null)),
        "honor":{"known_reward_honor":u64::try_from(honor).ok(),"unknown_reward_count":unknown_honor},
        "currency_choices":{"quests_with_observed_currency_choices":currency_choices,
            "unknown_currency_choice_list_count":unknown_currency_choices,"choices_applied":false},
        "choices_applied":false,"spell_rewards_projected":false,"reward_delivery_verified":false,
        "cross_quest_exclusivity_verified":false,"live":false});
    if include_bag_space {
        let quantities = report["fixed_items"]["known_fixed_quantity_by_item_id"].as_object().unwrap();
        let mut space = analysis::incoming_items_bag_space(snapshot,
            quantities.iter().map(|(id, quantity)|(id.parse().unwrap(),quantity.as_u64().unwrap())),
            "known_fixed_rewards_before_turn_in_without_item_consumption_or_choices", "known_fixed_reward_quantity");
        let complete = report["fixed_items"]["unknown_fixed_reward_list_count"] == 0;
        space["all_fixed_reward_lists_observed"] = json!(complete);
        space["unknown_fixed_reward_list_count"] = report["fixed_items"]["unknown_fixed_reward_list_count"].clone();
        space["all_fixed_rewards_estimated_new_stack_slots"] = if complete {space["estimated_new_stack_slots"].clone()} else {Value::Null};
        space["all_fixed_rewards_fit_recorded_general_slots"] = if complete {space["fits_recorded_general_slots"].clone()} else {Value::Null};
        space["all_fixed_rewards_estimated_new_stack_slots_if_same_item_stacks_merge"] = if complete {space["estimated_new_stack_slots_if_same_item_stacks_merge"].clone()} else {Value::Null};
        space["all_fixed_rewards_fit_recorded_general_slots_if_same_item_stacks_merge"] = if complete {space["fits_recorded_general_slots_if_same_item_stacks_merge"].clone()} else {Value::Null};
        space["choice_rewards_included"] = json!(false);
        space["quest_item_consumption_included"] = json!(false);
        space["reward_delivery_verified"] = json!(false);
        space["game_execution"] = json!(false);
        report["fixed_reward_bag_space"] = space;
    }
    report
}

impl Page {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.include_reward_bag_space || self.include_reward_review,
            "Reward bag space requires reward review");
        ensure!(
            (1..=50).contains(&self.limit),
            "Limit must be between 1 and 50"
        );
        ensure!(
            self.giver_id.is_none() || self.giver_kind.is_some(),
            "Giver ID requires giver kind"
        );
        if let Some(ids) = &self.quest_ids {
            ensure!(
                (1..=64).contains(&ids.len())
                    && ids.iter().collect::<BTreeSet<_>>().len() == ids.len(),
                "Quest IDs must contain 1 to 64 distinct IDs"
            );
        }
        Ok(())
    }
}

pub fn review(snapshot: &Snapshot, page: &Page) -> Result<Value> {
    page.validate()?;
    ensure!(
        snapshot.client.family == Family::Forever,
        "Quest turn-in review requires Forever"
    );
    let mut result = json!({
        "scope":"saved_provider_finisher_references", "status":"source_quests_not_recorded",
        "selected_quest_count":null, "ready_flag_unknown_quest_count":null,
        "reference_status_counts":null, "matching_group_count":null, "groups":null,
        "ungrouped_selected_quest_count":null, "ungrouped_quest_ids":null,
        "ungrouped_quest_ids_truncated":null, "selected_reference_truncated_group_count":null,
        "has_more":false, "offset":page.offset, "limit":page.limit,
        "live":false, "game_execution":false, "native_identity_verified":false,
        "current_readiness_verified":false, "turn_in_eligibility_verified":false,
        "optimal_route_verified":false, "complete_finisher_coverage_verified":false
    });
    if page.include_reward_review {
        result["unique_quest_reward_review"] = Value::Null;
        result["group_reward_views_are_additive"] = json!(false);
    }
    let references = snapshot.sections.get("quest_giver_references");
    if let Some(references) = references {
        quest_references::validate(references)?;
        quest_references::validate_source(references, snapshot.sections.get("quests"))?;
    }
    let Some(quests) = snapshot.sections.get("quests").and_then(Value::as_object) else {
        return bounded_result(result, page.include_reward_review);
    };
    let mut selected = BTreeMap::new();
    let mut unknown = 0;
    for (key, quest) in quests {
        let id: u32 = key.parse()?;
        ensure!(id > 0 && id.to_string() == *key, "Invalid saved quest ID");
        if page
            .quest_ids
            .as_ref()
            .is_some_and(|ids| !ids.iter().any(|wanted| wanted.get() == id))
        {
            continue;
        }
        let ready = quest.get("ready_for_turn_in").and_then(Value::as_bool);
        if ready.is_none() {
            unknown += 1;
        }
        if page
            .ready_for_turn_in
            .is_some_and(|wanted| ready != Some(wanted))
        {
            continue;
        }
        selected.insert(id, ready);
    }
    result["selected_quest_count"] = json!(selected.len());
    result["ready_flag_unknown_quest_count"] = json!(unknown);
    result["status"] = json!("references_not_recorded");
    let Some(references) = references else {
        return bounded_result(result, page.include_reward_review);
    };
    let entries = references["entries"].as_object().unwrap();
    let mut statuses = BTreeMap::from([
        ("reported", 0usize),
        ("unavailable", 0),
        ("not_in_provider", 0),
        ("not_captured", 0),
    ]);
    let mut groups: BTreeMap<(GiverKind, u32), BTreeSet<u32>> = BTreeMap::new();
    let mut ungrouped = Vec::new();
    let mut truncated = 0;
    for &id in selected.keys() {
        let row = entries.get(&id.to_string());
        let status = row
            .and_then(|row| row["status"].as_str())
            .unwrap_or("not_captured");
        *statuses.get_mut(status).unwrap() += 1;
        let mut grouped = false;
        for kind in [GiverKind::Npc, GiverKind::Object] {
            if page.giver_kind.is_some_and(|wanted| wanted != kind) {
                continue;
            }
            let group = row
                .and_then(|row| row.get("finished_by"))
                .and_then(|finished| finished.get(kind.source()));
            let Some(group) = group else {
                continue;
            };
            if group["truncated"] == true {
                truncated += 1;
            }
            for giver in group["entries"].as_object().unwrap().keys() {
                let giver: u32 = giver.parse()?;
                if page.giver_id.is_some_and(|wanted| wanted.get() != giver) {
                    continue;
                }
                groups.entry((kind, giver)).or_default().insert(id);
                grouped = true;
            }
        }
        if !grouped {
            ungrouped.push(id);
        }
    }
    let matching = groups.len();
    if page.include_reward_review {
        let unique_ids = groups
            .values()
            .flat_map(|ids| ids.iter().copied())
            .collect();
        result["unique_quest_reward_review"] = reward_review(snapshot, &unique_ids, page.include_reward_bag_space);
    }
    let rows: Vec<_> = groups.into_iter().skip(page.offset as usize).take(page.limit).map(|((kind,giver),ids)| {
        let by_ready = |ready| ids.iter().copied().filter(|id| selected[id] == ready).collect::<Vec<_>>();
        let mut group = json!({"giver_kind":kind.label(), "giver_id":giver, "quest_ids":ids,
            "ready_for_turn_in_quest_ids":by_ready(Some(true)), "not_ready_quest_ids":by_ready(Some(false)),
            "unknown_ready_quest_ids":by_ready(None)});
        if page.include_reward_review { group["reward_review"] = reward_review(snapshot, &ids, page.include_reward_bag_space); }
        group
    }).collect();
    result["status"] = json!("reviewed");
    result["reference_status_counts"] = json!(statuses);
    result["matching_group_count"] = json!(matching);
    result["groups"] = json!(rows);
    result["has_more"] = json!(matching.saturating_sub(page.offset as usize) > page.limit);
    result["ungrouped_selected_quest_count"] = json!(ungrouped.len());
    result["ungrouped_quest_ids_truncated"] = json!(ungrouped.len() > 64);
    result["ungrouped_quest_ids"] = json!(ungrouped.into_iter().take(64).collect::<Vec<_>>());
    result["selected_reference_truncated_group_count"] = json!(truncated);
    bounded_result(result, page.include_reward_review)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(value: Value) -> Page {
        serde_json::from_value(value).unwrap()
    }
    fn snapshot(mut quests: Value, rows: Value) -> Snapshot {
        for quest in quests.as_object_mut().unwrap().values_mut() {
            quest["title"] = json!("Fixture");
            quest["level"] = json!(1);
        }
        let snapshot: Snapshot = serde_json::from_value(
            json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"forever","build":"1","interface":1},
            "character":{"name":"Example","realm":"Test"},"sections":{"quests":quests,
                "quest_giver_references":{"coverage":"first_saved_quest_ids","provider":"questiedb",
                    "provider_mode":"source","provider_contract_version":1,
                    "reported_quest_count":quests.as_object().unwrap().len(),"entries":rows}}}),
        )
        .unwrap();
        snapshot.validate().unwrap();
        snapshot
    }
    fn group(ids: &[u32]) -> Value {
        json!({"entries":ids.iter().map(|id|(id.to_string(),json!(true))).collect::<BTreeMap<_,_>>(),
            "distinct_id_count":ids.len(),"truncated":false})
    }
    fn fixture() -> Snapshot {
        snapshot(
            json!({"2":{"ready_for_turn_in":true},"10":{"ready_for_turn_in":false},"30":{}}),
            json!({"2":{"status":"reported","finished_by":{"npcs":group(&[2,10]),"objects":group(&[2])}},
                "10":{"status":"reported","finished_by":{"npcs":group(&[2])}},
                "30":{"status":"reported","finished_by":{"npcs":group(&[2])}}}),
        )
    }
    #[test]
    fn shared_givers_preserve_namespaces_numeric_order_and_unknown_readiness() {
        let result = review(&fixture(), &page(json!({}))).unwrap();
        assert_eq!(result["matching_group_count"], 3);
        assert_eq!(
            result["groups"][0],
            json!({"giver_kind":"npc","giver_id":2,"quest_ids":[2,10,30],
            "ready_for_turn_in_quest_ids":[2],"not_ready_quest_ids":[10],"unknown_ready_quest_ids":[30]})
        );
        assert_eq!(result["groups"][1]["giver_id"], 10);
        assert_eq!(result["groups"][2]["giver_kind"], "object");
        assert_eq!(result["groups"][2]["giver_id"], 2);
        assert_eq!(result["ready_flag_unknown_quest_count"], 1);
        for flag in [
            "live",
            "game_execution",
            "native_identity_verified",
            "current_readiness_verified",
            "turn_in_eligibility_verified",
            "optimal_route_verified",
            "complete_finisher_coverage_verified",
        ] {
            assert_eq!(result[flag], false);
        }
    }
    #[test]
    fn filters_are_anded_and_false_does_not_select_unknown() {
        let result = review(&fixture(), &page(json!({"quest_ids":[10,30,999],"ready_for_turn_in":false,"giver_kind":"npc","giver_id":2}))).unwrap();
        assert_eq!(result["selected_quest_count"], 1);
        assert_eq!(result["ready_flag_unknown_quest_count"], 1);
        assert_eq!(result["groups"][0]["quest_ids"], json!([10]));
        let result = review(
            &fixture(),
            &page(json!({"ready_for_turn_in":false,"giver_kind":"object","giver_id":2})),
        )
        .unwrap();
        assert_eq!(result["groups"], json!([]));
        assert_eq!(result["ungrouped_quest_ids"], json!([10]));
    }
    #[test]
    fn pagination_counts_full_projection() {
        let result = review(&fixture(), &page(json!({"offset":1,"limit":1}))).unwrap();
        assert_eq!(result["matching_group_count"], 3);
        assert_eq!(result["groups"][0]["giver_id"], 10);
        assert_eq!(result["has_more"], true);
        let result = review(&fixture(), &page(json!({"offset":u32::MAX,"limit":1}))).unwrap();
        assert_eq!(result["groups"], json!([]));
        assert_eq!(result["has_more"], false);
    }
    #[test]
    fn empty_and_missing_sections_remain_distinct() {
        let result = review(&snapshot(json!({}), json!({})), &page(json!({}))).unwrap();
        assert_eq!(result["status"], "reviewed");
        assert_eq!(result["groups"], json!([]));
        assert_eq!(result["matching_group_count"], 0);
        let mut saved = fixture();
        saved.sections.remove("quest_giver_references");
        let result = review(&saved, &page(json!({}))).unwrap();
        assert_eq!(result["status"], "references_not_recorded");
        assert_eq!(result["selected_quest_count"], 3);
        assert_eq!(result["ready_flag_unknown_quest_count"], 1);
        assert!(result["groups"].is_null());
        assert!(result["reference_status_counts"].is_null());
        saved.sections.remove("quests");
        let result = review(&saved, &page(json!({}))).unwrap();
        assert_eq!(result["status"], "source_quests_not_recorded");
        assert!(result["selected_quest_count"].is_null());
    }
    #[test]
    fn uncaptured_prefix_statuses_and_bounded_ungrouped_ids() {
        let quests = Value::Object((1..=66).map(|id| (id.to_string(), json!({}))).collect());
        let rows = Value::Object(
            (1..=64)
                .map(|id| {
                    (
                        id.to_string(),
                        json!({"status":if id==1 {"not_in_provider"} else {"unavailable"}}),
                    )
                })
                .collect(),
        );
        let saved = snapshot(quests, rows);
        let result = review(&saved, &page(json!({}))).unwrap();
        assert_eq!(
            result["reference_status_counts"],
            json!({"reported":0,"unavailable":63,"not_in_provider":1,"not_captured":2})
        );
        assert_eq!(result["ungrouped_selected_quest_count"], 66);
        assert_eq!(result["ungrouped_quest_ids"].as_array().unwrap().len(), 64);
        assert_eq!(result["ungrouped_quest_ids"][63], 64);
        assert_eq!(result["ungrouped_quest_ids_truncated"], true);
        let result = review(&saved, &page(json!({"quest_ids":[66]}))).unwrap();
        assert_eq!(result["reference_status_counts"]["not_captured"], 1);
        assert_eq!(result["ungrouped_quest_ids"], json!([66]));
    }
    #[test]
    fn retained_truncation_counts_selected_kind_before_giver_filter() {
        let mut saved = snapshot(
            json!({"1":{}}),
            json!({"1":{"status":"reported","finished_by":{"npcs":group(&(1..=16).collect::<Vec<_>>()),"objects":group(&[99])}}}),
        );
        let reference = &mut saved.sections.get_mut("quest_giver_references").unwrap()["entries"]["1"]
            ["finished_by"]["npcs"];
        reference["distinct_id_count"] = json!(17);
        reference["truncated"] = json!(true);
        let result = review(&saved, &page(json!({"giver_kind":"npc","giver_id":99}))).unwrap();
        assert_eq!(result["groups"], json!([]));
        assert_eq!(result["selected_reference_truncated_group_count"], 1);
        let result = review(&saved, &page(json!({"giver_kind":"object"}))).unwrap();
        assert_eq!(result["selected_reference_truncated_group_count"], 0);
    }
    #[test]
    fn invalid_pages_family_and_reference_source_are_errors() {
        for bad in [
            json!({"giver_id":1}),
            json!({"quest_ids":[]}),
            json!({"quest_ids":[1,1]}),
            json!({"limit":0}),
            json!({"limit":51}),
            json!({"quest_ids":(1..=65).collect::<Vec<_>>()}),
        ] {
            assert!(page(bad).validate().is_err());
        }
        for bad in [
            json!({"quest_ids":[0]}),
            json!({"giver_kind":"item"}),
            json!({"extra":true}),
            json!({"offset":-1}),
        ] {
            assert!(serde_json::from_value::<Page>(bad).is_err());
        }
        let mut saved = fixture();
        saved.client.family = Family::Retail;
        assert!(review(&saved, &page(json!({}))).is_err());
        saved.client.family = Family::Forever;
        saved.sections.get_mut("quest_giver_references").unwrap()["entries"]["2"]["finished_by"]
            ["npcs"]["entries"] = json!([]);
        assert!(review(&saved, &page(json!({}))).is_err());
        let mut saved = fixture();
        saved.sections.get_mut("quest_giver_references").unwrap()["reported_quest_count"] =
            json!(4);
        assert!(review(&saved, &page(json!({}))).is_err());
    }

    fn reward_fixture() -> Snapshot {
        snapshot(
            json!({
            "2":{"ready_for_turn_in":true,"required_copper":0,"reward_amounts":{"xp":10,"copper":20,"honor":3},
                "item_rewards":{"1":{"item_id":1,"quantity":2}},"item_choices":{"1":{"item_id":1,"quantity":100}},
                "currency_rewards":{"1":{"currency_id":9,"quantity":4}},"currency_choices":{"1":{"currency_id":9,"quantity":100}},
                "renown_rewards":{"1":{"faction_id":5,"quantity":6}}},
            "10":{"ready_for_turn_in":false,"required_copper":5,"reward_amounts":{"xp":30,"copper":40,"honor":7},
                "item_rewards":{"1":{"item_id":1,"quantity":3}},"item_choices":{},"currency_rewards":{},"currency_choices":{},"renown_rewards":{}},
            "30":{}}),
            json!({
            "2":{"status":"reported","finished_by":{"npcs":group(&[2,10]),"objects":group(&[2])}},
            "10":{"status":"reported","finished_by":{"npcs":group(&[2])}},
            "30":{"status":"unavailable"}}),
        )
    }

    #[test]
    fn rewards_union_is_unique_and_independent_of_paging() {
        let saved = reward_fixture();
        let result = review(
            &saved,
            &page(json!({"include_reward_review":true,"limit":1})),
        )
        .unwrap();
        let union = &result["unique_quest_reward_review"];
        assert_eq!(union["quest_ids"], json!([2, 10]));
        assert_eq!(union["quest_count"], 2);
        assert_eq!(union["xp"]["known_reward_xp"], 40);
        assert_eq!(union["wallet"]["known_required_copper"], 5);
        assert_eq!(union["wallet"]["known_reward_copper"], 60);
        assert_eq!(union["honor"]["known_reward_honor"], 10);
        assert_eq!(
            union["currencies"]["known_fixed_rewards"]["9"]["proposed_quantity"],
            4
        );
        assert!(
            union["currencies"]["known_fixed_rewards"]["9"]["quantity_within_recorded_headroom"]
                .is_null()
        );
        assert_eq!(
            union["renown"]["known_rewards_by_faction"]["5"]["reward_points"],
            6
        );
        assert!(union["renown"]["known_rewards_by_faction"]["5"]["reaches_next_rank"].is_null());
        assert_eq!(
            union["fixed_items"]["known_fixed_quantity_by_item_id"]["1"],
            5
        );
        assert_eq!(
            union["currency_choices"]["quests_with_observed_currency_choices"],
            json!([2])
        );
        assert_eq!(
            union["currency_choices"]["unknown_currency_choice_list_count"],
            0
        );
        assert_eq!(union["choices_applied"], false);
        assert_eq!(union["currency_choices"]["choices_applied"], false);
        assert_eq!(result["groups"][0]["reward_review"], *union);
        assert_eq!(result["group_reward_views_are_additive"], false);
        let other = review(
            &saved,
            &page(json!({"include_reward_review":true,"offset":1,"limit":1})),
        )
        .unwrap();
        assert_eq!(other["groups"][0]["reward_review"]["quest_ids"], json!([2]));
        assert_eq!(other["unique_quest_reward_review"], *union);
        let beyond = review(
            &saved,
            &page(json!({"include_reward_review":true,"offset":u32::MAX})),
        )
        .unwrap();
        assert_eq!(beyond["groups"], json!([]));
        assert_eq!(beyond["unique_quest_reward_review"], *union);
        for flag in [
            "spell_rewards_projected",
            "reward_delivery_verified",
            "cross_quest_exclusivity_verified",
            "live",
        ] {
            assert_eq!(union[flag], false);
        }
    }

    #[test]
    fn reward_opt_in_is_strict_and_omission_preserves_shape() {
        let saved = fixture();
        let omitted = review(&saved, &page(json!({}))).unwrap();
        let disabled = review(&saved, &page(json!({"include_reward_review":false}))).unwrap();
        assert_eq!(omitted, disabled);
        assert!(omitted.get("unique_quest_reward_review").is_none());
        assert!(omitted.get("group_reward_views_are_additive").is_none());
        assert!(omitted["groups"][0].get("reward_review").is_none());
        for bad in [json!(null), json!("true"), json!(1), json!([])] {
            assert!(serde_json::from_value::<Page>(json!({"include_reward_review":bad})).is_err());
        }
        let mut saved = saved;
        saved.sections.remove("quest_giver_references");
        for without_source in [false, true] {
            if without_source {
                saved.sections.remove("quests");
            }
            let result = review(&saved, &page(json!({"include_reward_review":true}))).unwrap();
            assert!(result["unique_quest_reward_review"].is_null());
            assert_eq!(result["group_reward_views_are_additive"], false);
        }
        let result = review(
            &snapshot(json!({}), json!({})),
            &page(json!({"include_reward_review":true})),
        )
        .unwrap();
        assert_eq!(result["unique_quest_reward_review"]["quest_count"], 0);
        assert_eq!(
            result["unique_quest_reward_review"]["xp"]["known_reward_xp"],
            0
        );
    }

    #[test]
    fn reward_unknowns_zero_observations_and_readiness_keep_existing_semantics() {
        let mut saved = fixture();
        let opt = page(json!({"include_reward_review":true}));
        let result = review(&saved, &opt).unwrap();
        let reward = &result["unique_quest_reward_review"];
        assert_eq!(reward["xp"]["unknown_reward_count"], 3);
        assert!(reward["xp"]["xp_to_next_level"].is_null());
        assert_eq!(reward["wallet"]["unknown_cost_count"], 3);
        assert_eq!(reward["wallet"]["wallet_observed"], false);
        assert!(reward["wallet"]["ready_turn_in_order"].is_null());
        assert_eq!(reward["honor"]["unknown_reward_count"], 3);
        assert_eq!(
            reward["currency_choices"]["unknown_currency_choice_list_count"],
            3
        );
        saved.sections.insert("wallet".into(), json!({"copper":0}));
        saved.sections.insert(
            "experience".into(),
            json!({"status":"progress","current_xp":0,"level_threshold_xp":50,"xp_disabled":false}),
        );
        saved.validate().unwrap();
        let result = review(&saved, &opt).unwrap();
        assert_eq!(
            result["unique_quest_reward_review"]["wallet"]["wallet_observed"],
            true
        );
        assert_eq!(
            result["unique_quest_reward_review"]["xp"]["xp_to_next_level"],
            50
        );
        let mut saved = reward_fixture();
        saved.sections.insert("wallet".into(), json!({"copper":0}));
        saved.sections.insert(
            "experience".into(),
            json!({"status":"progress","current_xp":0,"level_threshold_xp":50,"xp_disabled":false}),
        );
        saved.sections.get_mut("quests").unwrap()["2"]["failed"] = json!(true);
        saved.validate().unwrap();
        let result = review(&saved, &opt).unwrap();
        let reward = &result["unique_quest_reward_review"];
        assert_eq!(reward["xp"]["known_reward_xp"], 40);
        assert!(reward["xp"]["ready_turn_in_plan"]["quest_ids_to_threshold"].is_null());
        assert!(reward["wallet"]["ready_turn_in_order"].is_null());
    }

    #[test]
    fn reward_output_cap_rejects_large_valid_source_without_truncation() {
        let quests = Value::Object(
            (1..=64)
                .map(|id| {
                    let items = Value::Object(
                        (1..=32)
                            .map(|slot| {
                                (
                                    slot.to_string(),
                                    json!({"item_id":id*100+slot,"quantity":1}),
                                )
                            })
                            .collect(),
                    );
                    (id.to_string(), json!({"item_rewards":items}))
                })
                .collect(),
        );
        let finishers = json!({"npcs":group(&(1..=16).collect::<Vec<_>>()),"objects":group(&(1..=16).collect::<Vec<_>>())});
        let rows = Value::Object(
            (1..=64)
                .map(|id| {
                    (
                        id.to_string(),
                        json!({"status":"reported","finished_by":finishers}),
                    )
                })
                .collect(),
        );
        let saved = snapshot(quests, rows);
        assert!(review(&saved, &page(json!({"limit":50}))).is_ok());
        let error = review(
            &saved,
            &page(json!({"include_reward_review":true,"limit":50})),
        )
        .unwrap_err();
        assert!(error.to_string().contains("256 KiB"));
        let bag_error = review(&saved, &page(json!({"include_reward_review":true,
            "include_reward_bag_space":true,"offset":u32::MAX}))).unwrap_err();
        assert!(bag_error.to_string().contains("256 KiB"));
        // Paging affects group output size, while the unique union remains complete.
        let result = review(
            &saved,
            &page(json!({"include_reward_review":true,"offset":u32::MAX})),
        )
        .unwrap();
        assert_eq!(result["unique_quest_reward_review"]["quest_count"], 64);
        assert_eq!(
            result["unique_quest_reward_review"]["fixed_items"]["known_fixed_quantity_by_item_id"]
                .as_object()
                .unwrap()
                .len(),
            2048
        );
    }

    #[test]
    fn zero_honor_is_observed_and_null_choices_are_unknown_lists() {
        let mut saved = reward_fixture();
        for id in ["2", "10"] {
            saved.sections.get_mut("quests").unwrap()[id]["reward_amounts"]["honor"] = json!(0);
            saved.sections.get_mut("quests").unwrap()[id]["currency_choices"] = Value::Null;
        }
        saved.validate().unwrap();
        let result = review(&saved, &page(json!({"include_reward_review":true}))).unwrap();
        let reward = &result["unique_quest_reward_review"];
        assert_eq!(reward["honor"]["known_reward_honor"], 0);
        assert_eq!(reward["honor"]["unknown_reward_count"], 0);
        assert_eq!(
            reward["currency_choices"]["unknown_currency_choice_list_count"],
            2
        );
    }
}
