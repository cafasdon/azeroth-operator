use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetRequest, compile},
};
use mlua::Lua;
use serde_json::{Value, json};

#[test]
fn use_transport_requires_explicit_empty_slots_and_preserves_exact_preimages()
-> Result<(), Box<dyn std::error::Error>> {
    let mut slots: serde_json::Map<String, Value> = (1..=19)
        .map(|i| (i.to_string(), json!({"ignored":true})))
        .collect();
    slots.insert("16".into(), json!({"ignored":false,"raw_item_id":123,"raw_location":100,
        "observed_item":{"kind":"bag","bag":0,"slot":1,"item_id":123,"guid":"Item-target","locked":false}}));
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture",
        "captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Fixture","realm":"Fixture"},"sections":{
        "equipment":{"16":{"item_id":123,"guid":"Item-old"}},
        "equipment_sets":{"coverage":"native_catalogue_raw_slots","can_use":true,"entries":{"0":{
        "name":"Fixture","icon":123,"is_equipped":false,"num_items":1,"num_equipped":0,"num_in_inventory":1,
        "num_lost":0,"num_ignored":18,"slots":slots}}}}}),
    )?;
    let mut request: EquipmentSetRequest =
        serde_json::from_value(json!({"operation":"use","set_id":0}))?;
    let sid = "a".repeat(64);
    assert!(compile(&snapshot, &sid, &request).is_err());
    request.expect_empty_slots = (1..=19).filter(|i| *i != 16).collect();
    let prepared = compile(&snapshot, &sid, &request)?;
    assert_eq!(prepared["addon_execution_enabled"], true);
    assert_eq!(prepared["requires_addon_version"], "0.11.0-dev");
    assert_eq!(prepared["equipment_before"]["16"], "Item-old");
    assert_eq!(prepared["equipment_before"]["17"], "");
    assert_eq!(prepared["desired_equipment"]["16"], "Item-target");
    let code = prepared["import_code"].as_str().unwrap();
    assert_eq!(code.split(':').count(), 212);
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    lua.load(include_str!("../addon/EquipmentSetImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.globals().set("code", code)?;
    lua.load(
        r#"
        op=AO.DecodeEquipmentSetOperation(code)
        assert(op.kind=='use_equipment_set' and op.set_id==0)
        assert(op.equipment_before['16']=='Item-old' and op.equipment_before['17']=='')
        assert(op.expected.slots['16'].observed_item.guid=='Item-target')
        assert(op.expected.slots['17'].ignored==true)
        assert(AO.ApplyEquipmentSetUse==nil) -- decoding does not install an executor
    "#,
    )
    .exec()?;
    lua.load(r#"
        function copy(t) local out={} for k,v in pairs(t) do out[k]=type(v)=='table' and copy(v) or v end return out end
        AO.family='retail'; AO.variant='live'; AO.interface=120105
        SlashCmdList={}; now=0; writes=0; delayed=false
        worn=copy(op.equipment_before); sets={['0']=copy(op.expected)}
        AO.collectors={equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)} end}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end; function UnitChannelInfo() end; function GetCursorInfo() end
        function GetTime() return now end
        function GetServerTime() return 0 end; function date() return '2025-01-01T00:00:00Z' end
        function IsInventoryItemLocked() return false end
        ItemLocation={CreateFromEquipmentSlot=function(_,i) return tostring(i) end}
        C_Item={DoesItemExist=function(i) return worn[i]~='' end,
            GetItemGUID=function(i) if worn[i]~='' then return worn[i] end end}
        C_EquipmentSet={EquipmentSetContainsLockedItems=function() return false end,
            UseEquipmentSet=function(id)
                assert(id==0 and AzerothOperatorOperations:find(op.id..':outcome_unknown:use_equipment_set;',1,true))
                writes=writes+1; if not delayed then worn['16']='Item-target' end
            end}
    "#).exec()?;
    for source in [
        include_str!("../addon/EquipmentSetUse.lua"),
        include_str!("../addon/Core.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(
        r#"
        AO.collectors.operation_receipts=function() return AO.ReadMacroReceipts() end
        assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code..'x') and writes==0)
        AO.ReviewMacroImport(code)
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and writes==1)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        AO.Capture()
    "#,
    )
    .exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let imported = azeroth_operator::import::decode(
        format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
    )?;
    assert_eq!(
        imported.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]["kind"],
        "use_equipment_set"
    );
    assert_eq!(
        imported.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]["status"],
        "applied"
    );
    lua.load(r#"
        -- Independent fixture attempts; production receipts are never cleared.
        AzerothOperatorOperations=''; worn=copy(op.equipment_before); delayed=true
        AO.ReviewMacroImport(code); local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='outcome_unknown' and writes==2)
        worn['16']='Item-target'; assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==2)
        AzerothOperatorOperations=''; worn=copy(op.equipment_before)
        AO.ReviewMacroImport(code); receipt,ok=AO.ApplyReviewedMacro(code)
        now=61; worn['16']='Item-target'
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==3)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==3)
    "#).exec()?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(
        r#"
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==3)
    "#,
    )
    .exec()?;
    let decoder: mlua::Function = ao.get("DecodeEquipmentSetOperation")?;
    for (index, replacement) in [
        (0, "AO-SET-1"),
        (9, "delete_equipment_set"),
        (193, "zz"),
        (208, "00"),
        (19, "1"),
    ] {
        let mut f: Vec<_> = code.split(':').collect();
        f[index] = replacement;
        assert!(decoder.call::<mlua::Value>(f.join(":")).is_err());
    }
    assert!(decoder.call::<mlua::Value>(format!("{code}:-")).is_err());
    request.expect_empty_slots.push(16);
    assert!(compile(&snapshot, &sid, &request).is_err());
    request.expect_empty_slots.pop();
    request.expect_empty_slots.push(1);
    assert!(compile(&snapshot, &sid, &request).is_err());
    request.expect_empty_slots.pop();
    let original = snapshot.clone();
    snapshot.sections.get_mut("equipment").unwrap()["16"]["guid"] = json!("Item-other-copy");
    assert_ne!(
        compile(&snapshot, &sid, &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot = original.clone();
    snapshot.sections.get_mut("equipment").unwrap()["16"]
        .as_object_mut()
        .unwrap()
        .remove("guid");
    assert!(compile(&snapshot, &sid, &request).is_err());
    for replacement in [
        json!({}),
        json!({"ignored":false}),
        json!({"ignored":false,"raw_item_id":0,"raw_location":-1}),
    ] {
        snapshot = original.clone();
        snapshot.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["slots"]["16"] =
            replacement;
        assert!(compile(&snapshot, &sid, &request).is_err());
    }
    snapshot = original;
    request.name = Some("Unexpected".into());
    assert!(compile(&snapshot, &sid, &request).is_err());
    Ok(())
}
