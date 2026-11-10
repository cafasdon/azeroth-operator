//! Deterministic grouping of historical provider finisher references only.
use crate::{Family, Snapshot, quest_references};
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
    pub ready_for_turn_in: Option<bool>,
    pub giver_kind: Option<GiverKind>,
    pub giver_id: Option<NonZeroU32>,
    pub quest_ids: Option<Vec<NonZeroU32>>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl Page {
    pub fn validate(&self) -> Result<()> {
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
    let references = snapshot.sections.get("quest_giver_references");
    if let Some(references) = references {
        quest_references::validate(references)?;
        quest_references::validate_source(references, snapshot.sections.get("quests"))?;
    }
    let Some(quests) = snapshot.sections.get("quests").and_then(Value::as_object) else {
        return Ok(result);
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
        return Ok(result);
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
    let rows: Vec<_> = groups.into_iter().skip(page.offset as usize).take(page.limit).map(|((kind,giver),ids)| {
        let by_ready = |ready| ids.iter().copied().filter(|id| selected[id] == ready).collect::<Vec<_>>();
        json!({"giver_kind":kind.label(), "giver_id":giver, "quest_ids":ids,
            "ready_for_turn_in_quest_ids":by_ready(Some(true)), "not_ready_quest_ids":by_ready(Some(false)),
            "unknown_ready_quest_ids":by_ready(None)})
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
    Ok(result)
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
}
