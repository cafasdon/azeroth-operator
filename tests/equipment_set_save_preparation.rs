use azeroth_operator::{
    Snapshot,
    equipment_set_operation::{EquipmentSetRequest, compile},
};
use serde_json::{Value, json};

#[test]
fn save_drafts_bind_catalogue_and_equipment_without_inferring_empty_slots()
-> Result<(), Box<dyn std::error::Error>> {
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
    assert!(draft["set_id"].is_null());
    assert!(
        draft["import_code"]
            .as_str()
            .unwrap()
            .starts_with("AO-SETSAVE-1:")
    );
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
    let lua = mlua::Lua::new();
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    for source in [
        include_str!("../addon/EquipmentSetImport.lua"),
        include_str!("../addon/EquipmentSetSaveImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.globals()
        .set("createCode", draft["import_code"].as_str().unwrap())?;
    lua.globals()
        .set("saveCode", saved["import_code"].as_str().unwrap())?;
    lua.load(r#"
        local c=AO.DecodeEquipmentSetSave(createCode)
        assert(c.kind=='create_equipment_set' and c.set_id==nil and c.name=='New' and c.icon==134400)
        assert(c.equipment_before['16'].guid=='Item-exact' and c.equipment_before['16'].item_id==123)
        assert(c.equipment_before['19'].guid=='' and c.equipment_before['19'].item_id==0)
        assert(c.ignored_slots['19'] and not c.ignored_slots['16'])
        assert(c.expected_catalogue['0'].assigned_spec==2)
        local s=AO.DecodeEquipmentSetSave(saveCode)
        assert(s.kind=='save_equipment_set' and s.set_id==0 and s.name=='Existing' and s.icon==123)
        assert(AO.ApplyEquipmentSetSave==nil)
    "#).exec()?;
    let decoder: mlua::Function = ao.get("DecodeEquipmentSetSave")?;
    let code = draft["import_code"].as_str().unwrap();
    let fields: Vec<_> = code.split(':').collect();
    for (index, value) in [
        (0, "AO-SET-1"),
        (1, "bad"),
        (3, "classic"),
        (9, "use_equipment_set"),
        (10, "0"),
        (12, "0"),
        (13, "2"),
        (14, "61"),
        (15, "1"),
        (16, "0"),
        (59, "2"),
        (71, "00"),
    ] {
        let mut bad = fields.clone();
        bad[index] = value;
        assert!(
            decoder.call::<mlua::Table>(bad.join(":")).is_err(),
            "field {index}"
        );
    }
    assert!(decoder.call::<mlua::Table>(format!("{code}:-")).is_err());
    assert!(
        decoder
            .call::<mlua::Table>(fields[..fields.len() - 1].join(":"))
            .is_err()
    );
    assert!(decoder.call::<mlua::Table>("x".repeat(524289)).is_err());
    let mut missing_target: Vec<_> = saved["import_code"].as_str().unwrap().split(':').collect();
    missing_target[10] = "9";
    assert!(
        decoder
            .call::<mlua::Table>(missing_target.join(":"))
            .is_err()
    );
    lua.load(r#"
        local c=AO.DecodeEquipmentSetSave(createCode)
        AO.family='retail'; AO.variant='live'; AO.interface=120105; SlashCmdList={}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return false end
        function GetServerTime() return 0 end; function date() return '2025-01-01T00:00:00Z' end
        AO.collectors={equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,entries=c.expected_catalogue} end}
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
    let mut large = snapshot.clone();
    for id in 1..32 {
        let mut other = entry.clone();
        other["name"] = json!(format!("Other{id}"));
        large.sections.get_mut("equipment_sets").unwrap()["entries"][id.to_string()] = other;
    }
    let large_draft = compile(&large, &sid, &parsed)?;
    let large_code = large_draft["import_code"].as_str().unwrap();
    assert!(large_code.len() > 8192);
    assert!(decoder.call::<mlua::Table>(large_code).is_ok());
    let mut duplicate: Vec<_> = large_code.split(':').collect();
    duplicate[252] = "0";
    assert!(decoder.call::<mlua::Table>(duplicate.join(":")).is_err());
    lua.load(r#"
        function copy(t) local out={} for k,v in pairs(t) do out[k]=type(v)=='table' and copy(v) or v end return out end
        function GetCursorInfo() end; function UnitCastingInfo() end; function UnitChannelInfo() end
        function StaticPopup_Visible() end; function IsInventoryItemLocked() return false end
        function GetTime() return now end
        ItemLocation={CreateFromEquipmentSlot=function(_,i) return i end}
        C_Item={DoesItemExist=function(i) return worn[tostring(i)].guid~='' end,
            GetItemGUID=function(i) local g=worn[tostring(i)].guid; if g~='' then return g end end,
            GetItemID=function(i) local id=worn[tostring(i)].item_id; if id~=0 then return id end end}
        function setup(code,mode)
            testCode=code; testMode=mode; op=AO.DecodeEquipmentSetSave(code)
            sets=copy(op.expected_catalogue); worn=copy(op.equipment_before); now=0; writes=0; flags={}
            for i=1,19 do flags[i]=false end
            AzerothOperatorOperations=''; MAX_EQUIPMENT_SETS_PER_PLAYER=20
            AO.collectors.equipment_sets=function() return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)} end
            AO.collectors.operation_receipts=function() return AO.ReadMacroReceipts() end
            C_EquipmentSet={GetEquipmentSetID=function(name) for id,e in pairs(sets) do if e.name==name then return tonumber(id) end end end,
                IsSlotIgnoredForSave=function(i) return flags[i] end,
                IgnoreSlotForSave=function(i) flags[i]=true end,UnignoreSlotForSave=function(i) flags[i]=false end,
                CreateEquipmentSet=function() native() end,SaveEquipmentSet=function() native() end}
        end
        function finish()
            local id=op.set_id or 7
            local target={name=op.name,icon=op.icon,is_equipped=true,num_items=1,num_equipped=1,num_in_inventory=0,num_lost=0,num_ignored=18,
                assigned_spec=op.set_id and sets[tostring(op.set_id)].assigned_spec,slots={}}
            for i=1,19 do local k=tostring(i); target.slots[k]={ignored=op.ignored_slots[k]} end
            target.slots['16']={ignored=false,raw_item_id=123,raw_location=16,
                observed_item={kind='equipment',slot=16,item_id=123,guid='Item-exact',locked=false}}
            sets[tostring(id)]=target
        end
        function native()
            assert(AzerothOperatorOperations:find(op.id..':outcome_unknown:'..op.kind..';',1,true))
            writes=writes+1
            if testMode~='delay' then finish() end
            if testMode=='throw_after' then error('post-effect failure') end
        end
    "#).exec()?;
    for source in [
        include_str!("../addon/EquipmentSetSaveState.lua"),
        include_str!("../addon/EquipmentSetSave.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    for (compiled, kind) in [
        (&draft, "create_equipment_set"),
        (&saved, "save_equipment_set"),
    ] {
        lua.globals()
            .set("testCode", compiled["import_code"].as_str().unwrap())?;
        lua.load(r#"
            setup(testCode,'success')
            assert(not pcall(AO.ApplyReviewedMacro,testCode) and writes==0)
            AO.ReviewMacroImport(testCode)
            assert(not pcall(AO.ApplyReviewedMacro,testCode..':-') and writes==0)
            AO.ReviewMacroImport(testCode); local r,ok=AO.ApplyReviewedMacro(testCode)
            assert(ok and r.status=='applied' and writes==1)
            AO.ReviewMacroImport(testCode); assert(not pcall(AO.ApplyReviewedMacro,testCode) and writes==1)
            AO.Capture()
        "#).exec()?;
        let exported: String = lua.globals().get("AzerothOperatorExport")?;
        let roundtrip = azeroth_operator::import::decode(
            format!("AzerothOperatorExport = \"{exported}\"").as_bytes(),
        )?;
        let id = compiled["operation_id"].as_str().unwrap();
        assert_eq!(
            roundtrip.sections["operation_receipts"][id],
            json!({"kind":kind,"status":"applied"})
        );
        let checkpoint: String = lua.globals().get("AzerothOperatorOperations")?;
        azeroth_operator::import::decode(format!("AzerothOperatorExport = \"{exported}\"\nAzerothOperatorOperations = \"{checkpoint}\"").as_bytes())?;
        lua.load(r#"
            setup(testCode,'delay'); AO.ReviewMacroImport(testCode); local r,ok=AO.ApplyReviewedMacro(testCode)
            assert(ok and r.status=='outcome_unknown' and writes==1)
            finish(); assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==1)
            setup(testCode,'delay'); AO.ReviewMacroImport(testCode); AO.ApplyReviewedMacro(testCode)
            now=61; finish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==1)
            setup(testCode,'throw_after'); AO.ReviewMacroImport(testCode); local r,ok=AO.ApplyReviewedMacro(testCode)
            assert(not ok and r.status=='outcome_unknown' and writes==1)
            assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
            AO.ReviewMacroImport(testCode); assert(not pcall(AO.ApplyReviewedMacro,testCode) and writes==1)
        "#).exec()?;
        lua.load(include_str!("../addon/MacroImport.lua"))
            .call::<()>(("AzerothOperator", ao.clone()))?;
        lua.load("AO.ReviewMacroImport(testCode); assert(not pcall(AO.ApplyReviewedMacro,testCode) and writes==1)").exec()?;
        lua.load("setup(testCode,'delay'); AO.ReviewMacroImport(testCode); AO.ApplyReviewedMacro(testCode)").exec()?;
        lua.load(include_str!("../addon/MacroImport.lua"))
            .call::<()>(("AzerothOperator", ao.clone()))?;
        lua.load(r#"
            finish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==1)
            AO.ReviewMacroImport(testCode); assert(not pcall(AO.ApplyReviewedMacro,testCode) and writes==1)
        "#).exec()?;
    }
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
    let empty_draft = compile(&changed, &sid, &parsed)?;
    assert!(
        decoder
            .call::<mlua::Table>(empty_draft["import_code"].as_str().unwrap())
            .is_ok()
    );
    Ok(())
}
