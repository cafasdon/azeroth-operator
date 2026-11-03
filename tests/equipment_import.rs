use azeroth_operator::{
    Snapshot,
    equipment_operation::{EquipmentRequest, compile},
};
use mlua::Lua;
use serde_json::json;

#[test]
fn equipment_transport_preserves_instance_preconditions() -> anyhow::Result<()> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Example","realm":"Test"},
        "sections":{"bags":{"0":{"1":{"item_id":123,"count":1,"guid":"Item-New"}}},"equipment":{"1":{"item_id":123,"guid":"Item-Old"}}}}),
    )?;
    let request = EquipmentRequest {
        bag: 0,
        slot: 1,
        target_slot: 1,
        expect_empty_target: false,
    };
    let compiled = compile(&snapshot, &"a".repeat(64), &request)?;
    let lua = Lua::new();
    let ao = lua.create_table().unwrap();
    lua.load(include_str!("../addon/EquipmentImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))
        .unwrap();
    let decoder: mlua::Function = ao.get("DecodeEquipmentOperation").unwrap();
    let op: mlua::Table = decoder
        .call(compiled["import_code"].as_str().unwrap())
        .unwrap();
    assert_eq!(op.get::<String>("source_guid").unwrap(), "Item-New");
    assert_eq!(op.get::<String>("target_guid").unwrap(), "Item-Old");
    assert_eq!(op.get::<u32>("item_id").unwrap(), 123);
    lua.globals().set("AO", ao.clone()).unwrap();
    lua.globals()
        .set("code", compiled["import_code"].as_str().unwrap())
        .unwrap();
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))
        .unwrap();
    lua.load(
        r#"
        AO.family='retail'; AO.variant='live'; AO.interface=120105
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Example' end
        function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        calls=0
        function AO.ApplyEquipmentOperation(op)
            calls=calls+1
            assert(op.source_guid=='Item-New' and op.target_guid=='Item-Old')
            assert(AzerothOperatorOperations:find('outcome_unknown:equip_bag_item',1,true))
            return {status='applied',kind=op.kind}
        end
        assert(not pcall(AO.ApplyReviewedMacro,code))
        assert(AO.ReviewMacroImport(code).kind=='equip_bag_item')
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and calls==1)
        assert(AzerothOperatorOperations:find('applied:equip_bag_item',1,true))
        AO.ReviewMacroImport(code)
        assert(not pcall(AO.ApplyReviewedMacro,code) and calls==1)
        -- A distinct attempted operation that errors keeps its uncertain receipt.
        local failing=code:gsub('AO%-EQUIP%-1:[0-9a-f]+:', 'AO-EQUIP-1:'..string.rep('f',64)..':',1)
        function AO.ApplyEquipmentOperation() calls=calls+1; error('simulated partial effect') end
        AO.ReviewMacroImport(failing)
        local uncertain,success=AO.ApplyReviewedMacro(failing)
        assert(not success and uncertain.status=='outcome_unknown' and calls==2)
        AO.ReviewMacroImport(failing)
        assert(not pcall(AO.ApplyReviewedMacro,failing) and calls==2)

    "#,
    )
    .exec()
    .unwrap();
    snapshot
        .sections
        .get_mut("equipment")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .clear();
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    let empty = EquipmentRequest {
        expect_empty_target: true,
        ..request
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &empty)?;
    let decoded: mlua::Table = decoder
        .call(prepared["import_code"].as_str().unwrap())
        .unwrap();
    assert_eq!(decoded.get::<String>("target_guid").unwrap(), "");
    snapshot.sections.get_mut("bags").unwrap()["0"]["1"]
        .as_object_mut()
        .unwrap()
        .remove("guid");
    assert!(compile(&snapshot, &"a".repeat(64), &empty).is_err());
    assert!(decoder.call::<mlua::Table>("AO-EQUIP-1:bad").is_err());
    Ok(())
}
