use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetRequest, compile},
};
use serde_json::{Value, json};

#[test]
fn save_drafts_bind_catalogue_and_equipment_without_inferring_empty_slots() -> anyhow::Result<()> {
    let slots: serde_json::Map<String, Value> = (1..=19)
        .map(|i| (i.to_string(), json!({"ignored":true})))
        .collect();
    let entry = json!({"name":"Existing","icon":123,"is_equipped":false,"num_items":0,"num_equipped":0,
        "num_in_inventory":0,"num_lost":0,"num_ignored":19,"assigned_spec":2,"slots":slots});
    let snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Fixture","realm":"Fixture"},
        "sections":{"equipment":{"16":{"item_id":123,"guid":"Item-exact"}},
        "equipment_sets":{"coverage":"native_catalogue_raw_slots","can_use":true,"entries":{"0":entry}}}}),
    )?;
    let empty: Vec<_> = (1..=19).filter(|i| *i != 16).collect();
    let sid = "a".repeat(64);
    let request = json!({"operation":"create","name":"New","icon":134400,"ignored_slots":empty,"expect_empty_slots":empty});
    let parsed: EquipmentSetRequest = serde_json::from_value(request.clone())?;
    let draft = compile(&snapshot, &sid, &parsed)?;
    assert_eq!(draft, compile(&snapshot, &sid, &parsed)?);
    assert_eq!(draft["kind"], "create_equipment_set");
    assert!(draft["set_id"].is_null() && draft["import_code"].is_null());
    assert_eq!(draft["addon_execution_enabled"], false);
    assert_eq!(
        draft["equipment_before"]["16"],
        json!({"guid":"Item-exact","item_id":123})
    );
    assert_eq!(
        draft["equipment_before"]["19"],
        json!({"guid":"","item_id":0})
    );
    assert_eq!(draft["desired_slots"]["19"], json!({"ignored":true}));
    assert_eq!(draft["ignored_slots"]["16"], false);
    let mut save = request.clone();
    save["operation"] = json!("save");
    save["set_id"] = json!(0);
    save.as_object_mut().unwrap().remove("name");
    save.as_object_mut().unwrap().remove("icon");
    let saved = compile(&snapshot, &sid, &serde_json::from_value(save.clone())?)?;
    assert_eq!(saved["name"], "Existing");
    assert_eq!(saved["icon"], 123);
    assert_eq!(saved["assigned_spec"], 2);
    assert_eq!(saved["overwrites_saved_contents"], true);
    for (key, value) in [
        ("set_id", json!(0)),
        ("name", json!("existing")),
        ("icon", json!(0)),
        ("spec_index", json!(1)),
        ("ignored_slots", json!([])),
        ("ignored_slots", json!([1, 1])),
        ("ignored_slots", json!([20])),
        ("expect_empty_slots", json!([])),
        ("expect_empty_slots", json!([1, 1])),
    ] {
        let mut bad = request.clone();
        bad[key] = value;
        assert!(
            compile(&snapshot, &sid, &serde_json::from_value(bad)?).is_err(),
            "{key}"
        );
    }
    for key in ["name", "icon", "ignored_slots"] {
        let mut bad = request.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(compile(&snapshot, &sid, &serde_json::from_value(bad)?).is_err());
    }
    save["name"] = json!("Rename");
    assert!(compile(&snapshot, &sid, &serde_json::from_value(save)?).is_err());
    for bad in [
        json!({"operation":"delete"}),
        json!({"operation":"use"}),
        json!({"operation":"assign_spec","spec_index":1}),
        json!({"operation":"delete","set_id":0,"ignored_slots":[]}),
    ] {
        assert!(compile(&snapshot, &sid, &serde_json::from_value(bad)?).is_err());
    }
    let mut changed = snapshot.clone();
    changed.sections.get_mut("equipment").unwrap()["16"]["guid"] = json!("Item-other-instance");
    assert_ne!(
        draft["operation_id"],
        compile(&changed, &sid, &parsed)?["operation_id"]
    );
    changed = snapshot.clone();
    changed.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["icon"] = json!(124);
    assert_ne!(
        draft["operation_id"],
        compile(&changed, &sid, &parsed)?["operation_id"]
    );
    changed = snapshot.clone();
    changed.sections.get_mut("equipment").unwrap()["16"]
        .as_object_mut()
        .unwrap()
        .remove("guid");
    assert!(compile(&changed, &sid, &parsed).is_err());
    changed = snapshot.clone();
    changed.sections.get_mut("equipment_sets").unwrap()["entries"] = json!({});
    assert!(compile(&changed, &sid, &parsed).is_ok());
    Ok(())
}
