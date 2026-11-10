//! Differences in saved bulk recipe observations, never a learned-recipe audit.
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    #[schemars(range(min = 1, max = 50))]
    pub limit: u32,
}
fn default_limit() -> u32 { 20 }

fn metadata(section: Option<&Value>) -> Value {
    match section {
        Some(value) => json!({"status":"observed", "coverage":value["coverage"],
            "profession_id":value["profession_id"],
            "observed_recipe_count":value["entries"].as_object().map(|rows|rows.len())}),
        None => json!({"status":"not_recorded", "coverage":null,
            "profession_id":null,"observed_recipe_count":null}),
    }
}

fn observation(row: &Value) -> Value {
    json!({"status":row["status"], "reagents":row["reagents"],
        "output_item_id":row["output_item_id"], "base_output_quantity":row["base_output_quantity"]})
}

/// Inputs come from validated owner-local snapshots, after identity/client/time checks.
pub fn compare(before: Option<&Value>, after: Option<&Value>, page: &Page) -> Result<Value> {
    ensure!((1..=50).contains(&page.limit), "Invalid recipe material comparison page");
    let mut report = json!({"status":"section_not_recorded", "before":metadata(before),
        "after":metadata(after), "counts":null, "total_changes":null,"rows":null,
        "offset":page.offset,"limit":page.limit,"next_offset":null,"has_more":null,
        "comparison_basis":"saved_bulk_status_fixed_inputs_and_base_output_only",
        "absence_means_unlearned":false,"complete_profession_coverage_verified":false,
        "recipe_definition_change_verified":false,"craftability_verified":false,
        "game_execution":false,"live":false});
    let Some((before, after)) = before.zip(after) else {return Ok(report)};
    if before["profession_id"] != after["profession_id"] {
        report["status"] = json!("profession_mismatch");
        return Ok(report);
    }
    let old = before["entries"].as_object().ok_or_else(||anyhow::anyhow!("Invalid recipe observations"))?;
    let new = after["entries"].as_object().ok_or_else(||anyhow::anyhow!("Invalid recipe observations"))?;
    let ids: BTreeSet<u32> = old.keys().chain(new.keys()).map(|id|id.parse())
        .collect::<std::result::Result<_,_>>()?;
    let (mut changed, mut unchanged, mut only_before, mut only_after) = (0,0,0,0);
    let mut rows = Vec::new();
    for id in ids {
        let key = id.to_string();
        let a = old.get(&key).map(observation);
        let b = new.get(&key).map(observation);
        let kind = match (&a,&b) {
            (Some(a),Some(b)) if a==b => {unchanged+=1;continue},
            (Some(_),Some(_)) => {changed+=1;"changed_observation"},
            (Some(_),None) => {only_before+=1;"observed_only_before"},
            (None,Some(_)) => {only_after+=1;"observed_only_after"},
            (None,None) => unreachable!(),
        };
        let fields: Vec<_> = if let (Some(a),Some(b))=(&a,&b) {
            ["status","reagents","output_item_id","base_output_quantity"].into_iter()
                .filter(|field| a[*field]!=b[*field]).collect()
        } else { Vec::new() };
        rows.push(json!({"recipe_id":id,"change_kind":kind,"before":a,"after":b,
            "changed_fields":fields}));
    }
    let total = rows.len();
    let end = (page.offset as usize).saturating_add(page.limit as usize).min(total);
    let more = end < total;
    report["status"] = json!("comparable");
    report["counts"] = json!({"changed_shared_recipes":changed,"unchanged_shared_recipes":unchanged,
        "observed_only_before":only_before,"observed_only_after":only_after});
    report["total_changes"] = json!(total);
    report["rows"] = json!(rows.into_iter().skip(page.offset as usize).take(page.limit as usize).collect::<Vec<_>>());
    report["has_more"] = json!(more);
    report["next_offset"] = if more {json!(end)} else {Value::Null};
    Ok(report)
}
