use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetRequest, compile},
};
use serde_json::{Value, json};

#[test]
fn association_drafts_bind_all_identities_and_report_displacement() -> anyhow::Result<()> {
    let slots: serde_json::Map<String, Value> =
        (1..=19).map(|i| (i.to_string(), json!({}))).collect();
    let mut entry = json!({"name":"Target","icon":123,"is_equipped":false,"num_items":0,"num_equipped":0,
        "num_in_inventory":0,"num_lost":0,"num_ignored":0,"assigned_spec":1,"slots":slots});
    let mut displaced = entry.clone();
    displaced["name"] = json!("Other");
    displaced["assigned_spec"] = json!(2);
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Fixture","realm":"Fixture"},
        "sections":{"equipment_sets":{"coverage":"native_catalogue_raw_slots","can_use":true,
        "specializations":{"1":101,"2":102,"3":103},"entries":{"0":entry,"1":displaced}}}}),
    )?;
    let mut request: EquipmentSetRequest =
        serde_json::from_value(json!({"operation":"assign_spec","set_id":0,"spec_index":2}))?;
    let sid = "a".repeat(64);
    let draft = compile(&snapshot, &sid, &request)?;
    assert_eq!(draft["displaced_set"]["set_id"], 1);
    assert_eq!(draft["expected_catalogue"]["0"]["assigned_spec"], 1);
    assert_eq!(draft["desired_catalogue"]["0"]["assigned_spec"], 2);
    assert!(
        draft["desired_catalogue"]["1"]
            .get("assigned_spec")
            .is_none()
    );
    assert_eq!(draft["addon_execution_enabled"], false);
    assert_eq!(draft["import_code"], Value::Null);
    assert_eq!(compile(&snapshot, &sid, &request)?, draft);
    snapshot.sections.get_mut("equipment_sets").unwrap()["specializations"]["3"] = json!(999);
    assert_ne!(
        compile(&snapshot, &sid, &request)?["operation_id"],
        draft["operation_id"]
    );
    request.spec_index = Some(1);
    assert_eq!(compile(&snapshot, &sid, &request)?["already_matches"], true);
    request.spec_index = Some(4);
    assert!(compile(&snapshot, &sid, &request).is_err());
    request.spec_index = None;
    assert!(compile(&snapshot, &sid, &request).is_err());
    request = serde_json::from_value(json!({"operation":"unassign_spec","set_id":0}))?;
    let unassign = compile(&snapshot, &sid, &request)?;
    assert!(
        unassign["desired_catalogue"]["0"]
            .get("assigned_spec")
            .is_none()
    );
    assert_eq!(unassign["desired_catalogue"]["1"]["assigned_spec"], 2);
    request.icon = Some(123);
    assert!(compile(&snapshot, &sid, &request).is_err());
    request.icon = None;
    entry["assigned_spec"] = json!(2);
    snapshot.sections.get_mut("equipment_sets").unwrap()["entries"]["0"] = entry;
    assert!(compile(&snapshot, &sid, &request).is_err());
    snapshot
        .sections
        .get_mut("equipment_sets")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("specializations");
    assert!(compile(&snapshot, &sid, &request).is_err());
    Ok(())
}
