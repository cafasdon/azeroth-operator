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
    assert_eq!(compiled["addon_execution_enabled"], false);
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
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let roundtrip = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(
        roundtrip.sections["equipment_sets"]["entries"]["0"],
        expected
    );
    lua.load("assert(AO.ApplyEquipmentSetOperation(op).status=='applied' and writes==1); assert(sets['0'].name=='New name')").exec()?;
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
    lua.load("op=AO.DecodeEquipmentSetOperation(deletion); sets['0']=copy(op.expected); assert(AO.ApplyEquipmentSetOperation(op).status=='applied' and writes==2 and sets['0']==nil)").exec()?;
    request.set_id = 99;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.set_id = 0;
    snapshot.client.family = azeroth_operator::Family::Classic;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}
