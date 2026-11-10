use azeroth_operator::{
    Snapshot,
    recipe_restock_batch::{AdditionalTarget, plan},
};
use serde_json::json;
use std::num::NonZeroU32;

fn snapshot() -> Snapshot {
    let s:Snapshot=serde_json::from_value(json!({"schema_version":1,"source":"fixture",
        "captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"live","build":"70862","interface":16001},
        "character":{"name":"Fixture","realm":"Test"},"sections":{"bags":{"0":{
            "1":{"item_id":1,"count":5},"2":{"item_id":10,"count":1}}},
        "character_bank":{"6":{"1":{"item_id":10,"count":100}}},
        "profession_recipe_materials":{"coverage":"reported_known_spell_schematics","profession_id":164,"entries":{
            "100":{"status":"fixed_items","reagents":{"1":2},"output_item_id":10,"base_output_quantity":{"minimum":2,"maximum":3}},
            "200":{"status":"fixed_items","reagents":{"10":3},"output_item_id":20,"base_output_quantity":{"minimum":1,"maximum":1}}
        }}},"unavailable":{}})).unwrap();
    s.validate().unwrap();
    s
}
fn number(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}
fn extra(id: u32, target: u32) -> AdditionalTarget {
    AdditionalTarget {
        recipe_id: number(id),
        target_bag_quantity: number(target),
    }
}

#[test]
fn downstream_consumption_preserves_both_targets_and_credits_producer_once() {
    let s = snapshot();
    let before = serde_json::to_value(&s).unwrap();
    let r = plan(&s, Some(number(100)), number(2), &[extra(200, 2)]).unwrap();
    assert_eq!(r["targets"][0]["planned_crafts"], 4);
    assert_eq!(r["targets"][1]["planned_crafts"], 2);
    assert_eq!(r["targets"][0]["other_requested_recipes_consume"], 6);
    assert_eq!(
        r["targets"][0]["modelled_final_bag_quantity"],
        json!({"minimum":3,"maximum":7})
    );
    assert_eq!(r["suggested_recipe_order"], json!([100, 200]));
    assert_eq!(r["combined_gross_materials"][0]["required"], 8);
    assert_eq!(r["ordered_materials"][0]["shortfall_vs_bags"], 3);
    assert_eq!(r["ordered_materials"][1]["required_before_batch"], 0);
    assert_eq!(r["game_execution"], false);
    assert_eq!(r["craftability_verified"], false);
    assert_eq!(before, serde_json::to_value(&s).unwrap());
    let reversed = plan(&s, Some(number(200)), number(2), &[extra(100, 2)]).unwrap();
    assert_eq!(
        reversed["suggested_recipe_order"],
        r["suggested_recipe_order"]
    );
    assert_eq!(reversed["targets"][0]["planned_crafts"], 2);
    assert_eq!(reversed["targets"][1]["planned_crafts"], 4);
}

#[test]
fn cycle_counts_do_not_invent_an_execution_order_or_initial_seed() {
    let mut s = snapshot();
    s.sections
        .get_mut("bags")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .clear();
    let rows = &mut s.sections.get_mut("profession_recipe_materials").unwrap()["entries"];
    rows["100"]["reagents"] = json!({"20":1});
    rows["200"]["reagents"] = json!({"10":1});
    rows["200"]["base_output_quantity"] = json!({"minimum":2,"maximum":2});
    s.validate().unwrap();
    let r = plan(&s, Some(number(100)), number(1), &[extra(200, 1)]).unwrap();
    assert_eq!(r["dependency_order_status"], "cyclic_or_downstream");
    assert!(r["suggested_recipe_order"].is_null());
    assert!(r["ordered_materials"].is_null());
    assert_eq!(r["targets"][0]["planned_crafts"], 1);
    assert_eq!(r["targets"][1]["planned_crafts"], 1);
    assert_eq!(r["targets"][0]["modelled_final_bag_quantity"]["minimum"], 1);
    assert_eq!(r["targets"][1]["modelled_final_bag_quantity"]["minimum"], 1);
    s.sections.get_mut("profession_recipe_materials").unwrap()["entries"]["100"]["base_output_quantity"] =
        json!({"minimum":1,"maximum":1});
    s.sections.get_mut("profession_recipe_materials").unwrap()["entries"]["200"]["base_output_quantity"] =
        json!({"minimum":1,"maximum":1});
    assert!(plan(&s, Some(number(100)), number(1), &[extra(200, 1)]).is_err());
}

#[test]
fn shared_reagents_are_counted_once_against_combined_need() {
    let mut s = snapshot();
    let rows = &mut s.sections.get_mut("profession_recipe_materials").unwrap()["entries"];
    rows["100"]["base_output_quantity"] = json!({"minimum":1,"maximum":1});
    rows["200"]["reagents"] = json!({"1":3});
    s.sections.get_mut("bags").unwrap()["0"]
        .as_object_mut()
        .unwrap()
        .remove("2");
    s.validate().unwrap();
    let r = plan(&s, Some(number(100)), number(2), &[extra(200, 2)]).unwrap();
    assert_eq!(r["combined_gross_materials"][0]["required"], 10);
    assert_eq!(
        r["combined_gross_materials"][0]["quantity_by_storage"]["bags"],
        5
    );
    assert_eq!(r["combined_gross_materials"][0]["shortfall_vs_bags"], 5);
}

#[test]
fn same_item_consumption_uses_net_yield_before_joint_target_math() {
    let mut s = snapshot();
    s.sections.get_mut("profession_recipe_materials").unwrap()["entries"]["100"]["reagents"]["10"] =
        json!(1);
    s.validate().unwrap();
    let r = plan(&s, Some(number(100)), number(2), &[extra(200, 2)]).unwrap();
    assert_eq!(r["targets"][0]["planned_crafts"], 7);
    assert_eq!(r["targets"][0]["same_recipe_consumption"], 7);
    assert_eq!(
        r["targets"][0]["modelled_final_bag_quantity"],
        json!({"minimum":2,"maximum":9})
    );
    assert_eq!(r["same_recipe_intermediate_output_credit"], false);
}

#[test]
fn zero_crafts_and_joint_bounds_remain_explicit() {
    let mut s = snapshot();
    s.sections.get_mut("bags").unwrap()["0"]["3"] = json!({"item_id":20,"count":3});
    let r = plan(&s, Some(number(100)), number(1), &[extra(200, 1)]).unwrap();
    assert_eq!(r["dependency_order_status"], "no_crafts_needed");
    assert_eq!(r["suggested_recipe_order"], json!([]));
    assert!(
        r["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["planned_crafts"] == 0 && row["recipe_review"].is_null())
    );
    assert!(plan(&s, Some(number(100)), number(1), &[]).is_err());
    assert!(plan(&s, Some(number(100)), number(1), &[extra(100, 1)]).is_err());
    assert!(plan(&s, Some(number(100)), number(1), &[extra(200, 1000)]).is_err());
    s.sections.get_mut("profession_recipe_materials").unwrap()["entries"]["200"]["output_item_id"] =
        json!(10);
    assert!(plan(&s, Some(number(100)), number(1), &[extra(200, 1)]).is_err());
}
