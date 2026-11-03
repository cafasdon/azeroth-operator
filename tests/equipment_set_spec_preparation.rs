use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetRequest, compile},
};
use serde_json::{Value, json};

#[test]
fn association_drafts_bind_all_identities_and_report_displacement()
-> Result<(), Box<dyn std::error::Error>> {
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
    let mut large = snapshot.clone();
    for id in 2..32 {
        let mut extra = entry.clone();
        extra.as_object_mut().unwrap().remove("assigned_spec");
        extra["name"] = json!(format!("Set {id} {}", "x".repeat(200)));
        large.sections.get_mut("equipment_sets").unwrap()["entries"][id.to_string()] = extra;
    }
    let large_op = compile(&large, &sid, &request)?;
    assert!(large_op["import_code"].as_str().unwrap().len() > 8192);
    assert_eq!(draft["displaced_set"]["set_id"], 1);
    assert_eq!(draft["expected_catalogue"]["0"]["assigned_spec"], 1);
    assert_eq!(draft["desired_catalogue"]["0"]["assigned_spec"], 2);
    assert!(
        draft["desired_catalogue"]["1"]
            .get("assigned_spec")
            .is_none()
    );
    assert_eq!(draft["addon_execution_enabled"], false);
    assert!(
        draft["import_code"]
            .as_str()
            .unwrap()
            .starts_with("AO-SETSPEC-1:")
    );
    let lua = mlua::Lua::new();
    lua.globals()
        .set("largeCode", large_op["import_code"].as_str().unwrap())?;
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    for source in [
        include_str!("../addon/EquipmentSetImport.lua"),
        include_str!("../addon/EquipmentSetSpecImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    let code = draft["import_code"].as_str().unwrap();
    lua.globals().set("code", code)?;
    lua.load(r#"
        op=AO.DecodeEquipmentSetSpec(code)
        assert(op.set_id==0 and op.spec_id==102 and op.spec_index==2 and op.displaced_set_id==1)
        assert(op.expected_specializations['3']==103 and op.expected_catalogue['0'].assigned_spec==1)
        assert(op.expected_catalogue['1'].assigned_spec==2 and op.expected_catalogue['1'].name=='Other')
        AO.family='retail'; AO.variant='live'; AO.interface=120105; SlashCmdList={}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return false end
        function GetServerTime() return 0 end; function date() return '2025-01-01T00:00:00Z' end
        AO.collectors={equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,
            entries=op.expected_catalogue,specializations=op.expected_specializations} end}
        assert(AO.ApplyEquipmentSetSpec==nil)
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.Capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let roundtrip = azeroth_operator::import::decode(
        format!("AzerothOperatorExport = \"{exported}\"").as_bytes(),
    )?;
    assert_eq!(
        roundtrip.sections["equipment_sets"],
        snapshot.sections["equipment_sets"]
    );
    lua.load(r#"
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        sets=copy(op.expected_catalogue); specMap=op.expected_specializations
        writes=0; now=0; delayed=false
        function GetTime() return now end; function GetCursorInfo() end
        function UnitCastingInfo() end; function UnitChannelInfo() end
        function GetNumSpecializations() return 3 end
        C_SpecializationInfo={GetSpecializationInfo=function(i) return specMap[tostring(i)] end}
        AO.collectors.equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets),specializations=specMap} end
        function finish(id,spec)
            for _,entry in pairs(sets) do if entry.assigned_spec==spec then entry.assigned_spec=nil end end
            sets[tostring(id)].assigned_spec=spec
        end
        C_EquipmentSet={GetEquipmentSetForSpec=function(spec) for id,entry in pairs(sets) do if entry.assigned_spec==spec then return tonumber(id) end end end,
            AssignSpecToEquipmentSet=function(id,spec)
                assert(AzerothOperatorOperations:find(op.id..':outcome_unknown:assign_equipment_set_spec;',1,true))
                writes=writes+1; if not delayed then finish(id,spec) end
            end,
            UnassignEquipmentSetSpec=function(id)
                assert(AzerothOperatorOperations:find(':outcome_unknown:unassign_equipment_set_spec;',1,true))
                writes=writes+1; sets[tostring(id)].assigned_spec=nil
            end}
    "#).exec()?;
    for source in [
        include_str!("../addon/EquipmentSetSpec.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(r#"
        assert(AO.ReviewMacroImport(largeCode).expected_catalogue['31'] and writes==0); AO.InvalidateMacroReview()
        AO.collectors.operation_receipts=function() return AO.ReadMacroReceipts() end
        assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code..':-') and writes==0)
        AO.ReviewMacroImport(code); local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and writes==1 and sets['0'].assigned_spec==2 and sets['1'].assigned_spec==nil)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        AO.Capture()
    "#).exec()?;
    let raw: String = lua.globals().get("AzerothOperatorExport")?;
    let applied =
        azeroth_operator::import::decode(format!("AzerothOperatorExport = \"{raw}\"").as_bytes())?;
    assert_eq!(
        applied.sections["operation_receipts"][draft["operation_id"].as_str().unwrap()]["status"],
        "applied"
    );
    lua.load(r#"
        -- New independent fixture attempt; never clear real receipts.
        AzerothOperatorOperations=''; sets=copy(op.expected_catalogue); delayed=true
        AO.ReviewMacroImport(code); local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='outcome_unknown' and writes==2)
        finish(0,2); assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==2)
        AzerothOperatorOperations=''; sets=copy(op.expected_catalogue)
        AO.ReviewMacroImport(code); receipt,ok=AO.ApplyReviewedMacro(code)
        now=61; finish(0,2); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==3)
        assert(not pcall(AO.ReviewMacroImport,'AO-MACRO-1:'..string.rep('a',8193)))
    "#).exec()?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown'); AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==3)").exec()?;
    lua.load(r#"
        AzerothOperatorOperations=''; sets=copy(op.expected_catalogue); delayed=false
        local native=C_EquipmentSet.AssignSpecToEquipmentSet
        C_EquipmentSet.AssignSpecToEquipmentSet=function(...) native(...); error('after native effect') end
        AO.ReviewMacroImport(code); local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(not ok and receipt.status=='outcome_unknown' and writes==4 and sets['0'].assigned_spec==2)
        assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==4)
    "#).exec()?;
    let decoder: mlua::Function = ao.get("DecodeEquipmentSetSpec")?;
    for (index, value) in [
        (0, "AO-SET-1"),
        (9, "delete_equipment_set"),
        (11, "4"),
        (12, "999"),
        (13, "65"),
        (15, "101"),
        (17, "129"),
        (18, "00"),
        (199, "0"),
        (208, "1"),
    ] {
        let mut f: Vec<_> = code.split(':').collect();
        f[index] = value;
        assert!(
            decoder.call::<mlua::Value>(f.join(":")).is_err(),
            "field {index}"
        );
    }
    assert!(decoder.call::<mlua::Value>(format!("{code}:-")).is_err());
    assert!(
        decoder
            .call::<mlua::Value>(&code[..code.len() - 2])
            .is_err()
    );
    assert!(decoder.call::<mlua::Value>("x".repeat(524289)).is_err());
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
    lua.globals()
        .set("unassignCode", unassign["import_code"].as_str().unwrap())?;
    lua.load("local u=AO.DecodeEquipmentSetSpec(unassignCode); assert(u.kind=='unassign_equipment_set_spec' and u.spec_index==nil and u.spec_id==nil)").exec()?;
    lua.load(r#"
        local u=AO.DecodeEquipmentSetSpec(unassignCode); sets=copy(u.expected_catalogue); specMap=u.expected_specializations
        AO.ReviewMacroImport(unassignCode); local receipt,ok=AO.ApplyReviewedMacro(unassignCode)
        assert(ok and receipt.kind=='unassign_equipment_set_spec' and receipt.status=='applied' and writes==5)
        assert(sets['0'].assigned_spec==nil and sets['1'].assigned_spec==2)
        AO.Capture()
    "#).exec()?;
    let raw: String = lua.globals().get("AzerothOperatorExport")?;
    let applied =
        azeroth_operator::import::decode(format!("AzerothOperatorExport = \"{raw}\"").as_bytes())?;
    assert_eq!(
        applied.sections["operation_receipts"][unassign["operation_id"].as_str().unwrap()]["kind"],
        "unassign_equipment_set_spec"
    );
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
