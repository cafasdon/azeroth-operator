use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetAction, EquipmentSetRequest, compile},
    import::decode,
};
use mlua::Lua;
use serde_json::json;

#[test]
fn set_transport_binds_all_metadata_slots_and_observed_identities()
-> Result<(), Box<dyn std::error::Error>> {
    let slots: serde_json::Map<String, serde_json::Value> =
        (1..=19).map(|i| (i.to_string(), json!({}))).collect();
    let mut expected = json!({"name":"Old: name|\n","icon":0,"is_equipped":false,"num_items":1,
        "num_equipped":0,"num_in_inventory":1,"num_lost":0,"num_ignored":1,"assigned_spec":2,"slots":slots});
    expected["slots"]["1"] = json!({"ignored":true,"raw_item_id":-1,"raw_location":-2});
    expected["slots"]["16"] = json!({"ignored":false,"raw_item_id":123,"raw_location":999,
        "observed_item":{"kind":"bag","bag":0,"slot":3,"item_id":123,"guid":"Item-Exact","locked":false}});
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Fixture","realm":"Fixture"},
        "sections":{"equipment_sets":{"coverage":"native_catalogue_raw_slots","can_use":true,"entries":{"0":expected}}}}),
    )?;
    let mut request = EquipmentSetRequest {
        operation: EquipmentSetAction::Modify,
        set_id: 0,
        name: Some("New name".into()),
        icon: Some(134400),
    };
    let compiled = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(compiled["addon_execution_enabled"], true);
    let code = compiled["import_code"].as_str().unwrap();
    assert_eq!(code.split(':').count(), 193);
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    lua.globals().set("code", code)?;
    lua.load(include_str!("../addon/EquipmentSetImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/EquipmentSetOperations.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(r#"
        op=AO.DecodeEquipmentSetOperation(code)
        assert(op.set_id==0 and op.expected.slots['1'].raw_item_id==-1)
        assert(op.expected.slots['16'].ignored==false and op.expected.slots['2'].ignored==nil)
        assert(op.expected.slots['16'].observed_item.bag==0)
        assert(op.expected.slots['16'].observed_item.locked==false)
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        AO.family='retail'; AO.variant='live'; AO.interface=120105
        SlashCmdList={}; writes=0; sets={['0']=copy(op.expected)}
        AO.collectors={equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,entries=sets} end}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end; function UnitChannelInfo() end; function GetCursorInfo() end
        function GetServerTime() return 0 end; function date() return '2025-01-01T00:00:00Z' end
        C_EquipmentSet={GetEquipmentSetID=function() end,
            ModifyEquipmentSet=function(id,name,icon) writes=writes+1; sets[tostring(id)].name=name; sets[tostring(id)].icon=tonumber(icon) end,
            DeleteEquipmentSet=function(id) writes=writes+1; sets[tostring(id)]=nil end}
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.collectors.operation_receipts=function() return AO.ReadMacroReceipts() end; now=0; function GetTime() return now end").exec()?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let roundtrip = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(
        roundtrip.sections["equipment_sets"]["entries"]["0"],
        expected
    );
    lua.load(r#"
        local modify=C_EquipmentSet.ModifyEquipmentSet
        C_EquipmentSet.ModifyEquipmentSet=function(...)
            assert(AzerothOperatorOperations:find(op.id..':outcome_unknown:modify_equipment_set;',1,true))
            return modify(...)
        end
        assert(not pcall(AO.ApplyReviewedMacro,code))
        AO.ReviewMacroImport(code)
        assert(not pcall(AO.ApplyReviewedMacro,code..'x') and writes==0)
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and writes==1 and sets['0'].name=='New name')
        AO.ReviewMacroImport(code)
        assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        C_EquipmentSet.ModifyEquipmentSet=modify
    "#).exec()?;
    let decoder: mlua::Function = ao.get("DecodeEquipmentSetOperation")?;
    for (index, replacement) in [
        (0, "AO-SET-2"),
        (3, "classic"),
        (9, "use_equipment_set"),
        (10, "00"),
        (12, "-"),
        (16, "20"),
        (22, "2"),
        (26, "1"),
    ] {
        let mut fields: Vec<&str> = code.split(':').collect();
        fields[index] = replacement;
        assert!(
            decoder.call::<mlua::Table>(fields.join(":")).is_err(),
            "field {index}"
        );
    }
    assert!(
        decoder
            .call::<mlua::Table>(format!("{code}:extra"))
            .is_err()
    );
    snapshot.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["slots"]["16"]["observed_item"]
        ["guid"] = json!("Item-Changed");
    assert_ne!(
        compiled["operation_id"],
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"]
    );
    assert!(decoder.call::<mlua::Table>("a".repeat(8193)).is_err());
    assert!(compile(&snapshot, "bad", &request).is_err());
    request.name = Some("bad|name".into());
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.name = Some("New name".into());
    request.icon = Some(0);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.icon = Some(134400);
    let mut collision = expected.clone();
    collision["name"] = json!("NEW NAME");
    snapshot.sections.get_mut("equipment_sets").unwrap()["entries"]["1"] = collision;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    snapshot.sections.get_mut("equipment_sets").unwrap()["entries"]
        .as_object_mut()
        .unwrap()
        .remove("1");
    request.operation = EquipmentSetAction::Delete;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.name = None;
    request.icon = None;
    let deletion = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(deletion["deletes_saved_set"], true);
    lua.globals()
        .set("deletion", deletion["import_code"].as_str().unwrap())?;
    lua.load(r#"
        op=AO.ReviewMacroImport(deletion); sets['0']=copy(op.expected)
        local receipt,ok=AO.ApplyReviewedMacro(deletion)
        assert(ok and receipt.status=='applied' and writes==2 and sets['0']==nil)
        AO.ReviewMacroImport(deletion); assert(not pcall(AO.ApplyReviewedMacro,deletion) and writes==2)
        AO.Capture()
    "#).exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let receipts: String = lua.globals().get("AzerothOperatorOperations")?;
    let confirmed = decode(
        format!(
            "AzerothOperatorExport = \"{exported}\"\nAzerothOperatorOperations = \"{receipts}\""
        )
        .as_bytes(),
    )?;
    assert_eq!(
        confirmed.sections["operation_receipts"][compiled["operation_id"].as_str().unwrap()]["status"],
        "applied"
    );
    assert_eq!(
        confirmed.sections["operation_receipts"][deletion["operation_id"].as_str().unwrap()]["kind"],
        "delete_equipment_set"
    );
    request.operation = EquipmentSetAction::Modify;
    request.name = Some("Later".into());
    request.icon = Some(123);
    for marker in ['b', 'c', 'd', 'e'] {
        let prepared = compile(&snapshot, &marker.to_string().repeat(64), &request)?;
        lua.globals().set(
            format!("code_{marker}"),
            prepared["import_code"].as_str().unwrap(),
        )?;
    }
    lua.load(r#"
        local native=C_EquipmentSet.ModifyEquipmentSet
        C_EquipmentSet.ModifyEquipmentSet=function(id,name,icon)
            writes=writes+1
            pending=function() sets[tostring(id)].name=name; sets[tostring(id)].icon=tonumber(icon) end
        end
        op=AO.ReviewMacroImport(code_b); sets['0']=copy(op.expected)
        local receipt,ok=AO.ApplyReviewedMacro(code_b)
        assert(ok and receipt.status=='outcome_unknown' and writes==3)
        AO.ReviewMacroImport(code_b); assert(not pcall(AO.ApplyReviewedMacro,code_b))
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
        pending(); now=1
        assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==3)
        op=AO.ReviewMacroImport(code_c); sets['0']=copy(op.expected)
        receipt,ok=AO.ApplyReviewedMacro(code_c); assert(ok and receipt.status=='outcome_unknown' and writes==4)
        pending(); now=62
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==4)
        C_EquipmentSet.ModifyEquipmentSet=native
        local executor=AO.ApplyEquipmentSetOperation
        AO.ApplyEquipmentSetOperation=nil; fallbackCalls=0
        AO.ApplyMacroOperation=function() fallbackCalls=fallbackCalls+1 end
        op=AO.ReviewMacroImport(code_d); sets['0']=copy(op.expected)
        receipt,ok=AO.ApplyReviewedMacro(code_d)
        assert(not ok and receipt.status=='outcome_unknown' and fallbackCalls==0 and writes==4)
        AO.ApplyEquipmentSetOperation=executor
        AO.ReviewMacroImport(code_d); assert(not pcall(AO.ApplyReviewedMacro,code_d) and writes==4)
        AO.ApplyEquipmentSetOperation=function() return nil end
        op=AO.ReviewMacroImport(code_e); sets['0']=copy(op.expected)
        receipt,ok=AO.ApplyReviewedMacro(code_e)
        assert(not ok and receipt.status=='outcome_unknown' and AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
        AO.ApplyEquipmentSetOperation=executor
        AO.ReviewMacroImport(code_e); assert(not pcall(AO.ApplyReviewedMacro,code_e) and writes==4)
    "#).exec()?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(
        r#"
        local op=AO.ReviewMacroImport(code_c)
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
        assert(not pcall(AO.ApplyReviewedMacro,code_c) and writes==4)
        op=AO.ReviewMacroImport(code_e)
        assert(not pcall(AO.ApplyReviewedMacro,code_e) and writes==4)
    "#,
    )
    .exec()?;
    request.set_id = 99;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.set_id = 0;
    snapshot.client.family = azeroth_operator::Family::Classic;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}
