use azeroth_operator::{
    Snapshot,
    bag_operation::{BagRequest, compile},
};
use mlua::Lua;
use serde_json::json;
#[test]
fn swap_transport_binds_both_items_and_rejects_ambiguous_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
 "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},
 "sections":{"bags":{"0":{"1":{"item_id":123,"count":2,"guid":"one","link":"source:|link"},"2":{"item_id":456,"count":3,"guid":"two","link":"target:|link"}}}}}),
    )?;
    let mut request: BagRequest = serde_json::from_value(
        json!({"operation":"swap","bag":0,"slot":1,"target_bag":0,"target_slot":2}),
    )?;
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(prepared["kind"], "swap_bag_items");
    assert_eq!(prepared["addon_execution_enabled"], false);
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.load(include_str!("../addon/BagSwapImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeBagSwap")?;
    let code = prepared["import_code"].as_str().unwrap();
    let op: mlua::Table = decoder.call(code)?;
    assert_eq!(
        op.get::<mlua::Table>("source")?.get::<String>("link")?,
        "source:|link"
    );
    assert_eq!(
        op.get::<mlua::Table>("target")?.get::<String>("guid")?,
        "two"
    );
    for (field, bad) in [
        (9, "00"),
        (10, "0"),
        (11, "4294967296"),
        (13, ""),
        (14, "zz"),
        (17, "123"),
        (19, "6f6e65"),
    ] {
        let mut fields: Vec<_> = code.split(':').collect();
        fields[field] = bad;
        assert!(decoder.call::<mlua::Table>(fields.join(":")).is_err());
    }
    assert!(
        decoder
            .call::<mlua::Table>(format!("{code}:extra"))
            .is_err()
    );
    request.expect_empty_target = true;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.expect_empty_target = false;
    request.amount = Some(1);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.amount = None;
    request.target_slot = 3;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.target_slot = 2;
    snapshot.sections.get_mut("bags").unwrap()["0"]["2"]["count"] = json!(4);
    assert_ne!(
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot.sections.get_mut("bags").unwrap()["0"]["2"]["link"] = json!(null);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}
