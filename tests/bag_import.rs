use azeroth_operator::{
    Snapshot,
    bag_operation::{BagRequest, compile},
};
use mlua::Lua;
use serde_json::json;
#[test]
fn full_stack_transport_requires_explicit_empty_target() -> anyhow::Result<()> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},
        "sections":{"bags":{"0":{"1":{"item_id":123,"count":20,"guid":"Item-Stack"}}}}}),
    )?;
    let mut request = BagRequest {
        bag: 0,
        slot: 1,
        target_bag: 0,
        target_slot: 2,
        expect_empty_target: false,
    };
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.expect_empty_target = true;
    let compiled = compile(&snapshot, &"a".repeat(64), &request)?;
    let lua = Lua::new();
    let ao = lua.create_table().unwrap();
    lua.load(include_str!("../addon/BagImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))
        .unwrap();
    let decode: mlua::Function = ao.get("DecodeBagOperation").unwrap();
    let op: mlua::Table = decode
        .call(compiled["import_code"].as_str().unwrap())
        .unwrap();
    assert_eq!(op.get::<String>("source_guid").unwrap(), "Item-Stack");
    assert_eq!(op.get::<u32>("count").unwrap(), 20);
    assert_eq!(op.get::<u32>("target_slot").unwrap(), 2);
    assert!(op.get::<bool>("expect_empty_target").unwrap());
    request.target_slot = 1;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.target_slot = 2;
    snapshot.sections.get_mut("bags").unwrap()["0"]["2"] =
        json!({"item_id":123,"count":1,"guid":"Other"});
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    snapshot.sections.get_mut("bags").unwrap()["0"]
        .as_object_mut()
        .unwrap()
        .remove("2");
    snapshot.sections.get_mut("bags").unwrap()["0"]["1"]
        .as_object_mut()
        .unwrap()
        .remove("guid");
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    assert!(decode.call::<mlua::Table>("AO-BAG-1:bad").is_err());
    Ok(())
}
