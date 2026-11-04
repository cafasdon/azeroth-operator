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
        operation: Default::default(),
        amount: None,
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
    lua.globals().set("AO", ao.clone()).unwrap();
    lua.globals()
        .set("code", compiled["import_code"].as_str().unwrap())
        .unwrap();
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao))
        .unwrap();
    lua.load(
        r#"
        AO.family='retail'; AO.variant='live'; AO.interface=120105; calls=0
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function AO.ApplyBagOperation(op)
            calls=calls+1
            assert(op.count==20 and op.source_guid=='Item-Stack')
            assert(AzerothOperatorOperations:find('outcome_unknown:move_bag_item',1,true))
            return {status='applied',kind=op.kind}
        end
        assert(not pcall(AO.ApplyReviewedMacro,code))
        assert(AO.ReviewMacroImport(code).kind=='move_bag_item')
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and calls==1)
        AO.ReviewMacroImport(code)
        assert(not pcall(AO.ApplyReviewedMacro,code) and calls==1)
        local failing=code:gsub('AO%-BAG%-1:[0-9a-f]+:', 'AO-BAG-1:'..string.rep('f',64)..':',1)
        function AO.ApplyBagOperation() calls=calls+1; error('partial placement') end
        AO.ReviewMacroImport(failing)
        local uncertain,success,detail=AO.ApplyReviewedMacro(failing)
        assert(not success and uncertain.status=='outcome_unknown' and calls==2)
        assert(detail:find('partial placement',1,true))
        assert(AzerothOperatorOperations:find('outcome_unknown:move_bag_item',1,true))
        AO.ReviewMacroImport(failing)
        assert(not pcall(AO.ApplyReviewedMacro,failing) and calls==2)

    "#,
    )
    .exec()
    .unwrap();
    request.amount = Some(5);
    let split = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(split["kind"], "split_bag_item");
    assert_ne!(split["operation_id"], compiled["operation_id"]);
    let decoded: mlua::Table = decode.call(split["import_code"].as_str().unwrap()).unwrap();
    assert_eq!(decoded.get::<u32>("amount").unwrap(), 5);
    assert_eq!(decoded.get::<String>("kind").unwrap(), "split_bag_item");
    lua.globals()
        .set("splitcode", split["import_code"].as_str().unwrap())
        .unwrap();
    lua.load(
        r#"
        function AO.ApplyBagOperation(op)
            assert(op.kind=='split_bag_item' and op.amount==5 and op.count==20)
            return {kind=op.kind,status='applied'}
        end
        AO.ReviewMacroImport(splitcode)
        local receipt,ok=AO.ApplyReviewedMacro(splitcode)
        assert(ok and receipt.kind=='split_bag_item')
        assert(AzerothOperatorOperations:find('applied:split_bag_item',1,true))
    "#,
    )
    .exec()
    .unwrap();
    for amount in [0, 20, 21] {
        request.amount = Some(amount);
        assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    }
    request.amount = None;
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
