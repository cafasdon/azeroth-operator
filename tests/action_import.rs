use azeroth_operator::{
    Snapshot,
    action_operation::{ActionRequest, compile},
};
use mlua::Lua;
use serde_json::json;
#[test]
fn compiler_decoder_bind_slots_and_macro_contents() -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
 "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},
 "sections":{"action_bar_observation":{"coverage":"loaded_builtin_buttons_only","observed_buttons":3,"missing_buttons":0,
 "slots":{"1":{"empty":false,"kind":"spell","id":123},"2":{"empty":false,"kind":"macro","id":121},"3":{"empty":true}}},
 "macros":{"121":{"name":"Macro","icon":134400,"body":"/cast Test\n/say test: value | literal","scope":"character"}}}}),
    )?;
    let request = ActionRequest {
        operation: Default::default(),
        source_slot: 1,
        target_slot: 2,
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.load(include_str!("../addon/ActionImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeActionOperation")?;
    let code = prepared["import_code"].as_str().unwrap();
    lua.globals().set("AO", ao.clone())?;
    lua.globals().set("code", code)?;
    lua.globals().set(
        "macroBody",
        snapshot.sections["macros"]["121"]["body"].as_str().unwrap(),
    )?;
    lua.load(
        r#"
        AO.family='retail'; AO.variant='live'; AO.interface=120105; writes=0
        slots={[1]={kind='spell',id=123},[2]={kind='macro',id=121}}
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end; function UnitChannelInfo() end
        function GetActionInfo(slot) local a=slots[slot]; if a then return a.kind,a.id end end
        function GetMacroInfo() return 'Macro',134400,macroBody end
        function GetCursorInfo() if cursor then return cursor.kind,cursor.id,nil,cursor.id end end
        AO.collectors={action_bar_observation=function() return {slots={['1']={},['2']={}}} end}
        function PickupAction(slot) writes=writes+1; cursor=slots[slot]; slots[slot]=nil end
        function PlaceAction(slot) writes=writes+1; slots[slot],cursor=cursor,slots[slot] end
    "#,
    )
    .exec()?;
    for source in [
        include_str!("../addon/ActionOperations.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(
        r#"
        assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code)
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.kind=='move_action_slot' and receipt.status=='applied' and writes==3)
        assert(slots[1].id==121 and slots[2].id==123 and cursor==nil)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==3)
    "#,
    )
    .exec()?;
    let checkpoint: String = lua.globals().get("AzerothOperatorOperations")?;
    lua.load(
        r#"
        SlashCmdList={}
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        AO.collectors.action_bar_observation=nil
        AO.collectors.operation_receipts=AO.ReadMacroReceipts
    "#,
    )
    .exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let imported = azeroth_operator::import::decode(
        format!("AzerothOperatorExport = \"{hex}\"\nAzerothOperatorOperations = \"{checkpoint}\"")
            .as_bytes(),
    )?;
    assert_eq!(
        imported.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]["status"],
        "applied"
    );
    let op: mlua::Table = decoder.call(code)?;
    assert_eq!(op.get::<u32>("source_slot")?, 1);
    let target: mlua::Table = op.get("target")?;
    let m: mlua::Table = target.get("macro")?;
    assert_eq!(
        m.get::<String>("body")?,
        "/cast Test\n/say test: value | literal"
    );
    assert_eq!(prepared["mode"], "swap");
    let moved = compile(
        &snapshot,
        &"a".repeat(64),
        &ActionRequest {
            operation: Default::default(),
            source_slot: 1,
            target_slot: 3,
        },
    )?;
    assert_eq!(moved["mode"], "move");
    lua.globals()
        .set("moveCode", moved["import_code"].as_str().unwrap())?;
    lua.load(r#"
        now=0; function GetTime() return now end
        slots={[1]={kind='spell',id=123}}; cursor=nil; writes=0
        AO.collectors.action_bar_observation=function() return {slots={['1']={},['3']={}}} end
        function PlaceAction(slot) writes=writes+1; pending=cursor; cursor=nil end
        AO.ReviewMacroImport(moveCode)
        local receipt,ok=AO.ApplyReviewedMacro(moveCode)
        local id=AO.DecodeActionOperation(moveCode).id
        assert(ok and receipt.status=='outcome_unknown' and writes==2)
        assert(AO.ReadMacroReceipts()[id].status=='outcome_unknown')
        slots[3]=pending; now=10
        assert(AO.ReadMacroReceipts()[id].status=='applied' and writes==2)
        local late=moveCode:gsub('AO%-ACTION%-1:[0-9a-f]+:','AO-ACTION-1:'..string.rep('f',64)..':',1)
        slots={[1]={kind='spell',id=123}}; now=20
        AO.ReviewMacroImport(late); AO.ApplyReviewedMacro(late)
        slots[3]=pending; now=81
        assert(AO.ReadMacroReceipts()[string.rep('f',64)].status=='outcome_unknown' and writes==4)
        AO.ReviewMacroImport(late); assert(not pcall(AO.ApplyReviewedMacro,late) and writes==4)
    "#).exec()?;

    let op: mlua::Table = decoder.call(moved["import_code"].as_str().unwrap())?;
    assert!(op.get::<mlua::Table>("target")?.get::<bool>("empty")?);
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                operation: Default::default(),
                source_slot: 1,
                target_slot: 4
            }
        )
        .is_err()
    );
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                operation: Default::default(),
                source_slot: 3,
                target_slot: 1
            }
        )
        .is_err()
    );
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                operation: Default::default(),
                source_slot: 1,
                target_slot: 1
            }
        )
        .is_err()
    );
    for (field, bad) in [
        (9, "01"),
        (10, "4097"),
        (11, "flyout"),
        (18, "0"),
        (20, "zz"),
    ] {
        let mut f: Vec<_> = code.split(':').collect();
        f[field] = bad;
        assert!(decoder.call::<mlua::Table>(f.join(":")).is_err());
    }
    snapshot.sections.get_mut("macros").unwrap()["121"]["body"] = json!("changed");
    assert_ne!(
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot.sections.get_mut("macros").unwrap()["121"]["body"] = json!("x".repeat(5000));
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    snapshot.sections.remove("macros");
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}

#[test]
fn copy_transport_preserves_source_and_refuses_occupied_target()
-> Result<(), Box<dyn std::error::Error>> {
    use azeroth_operator::action_operation::ActionOperation;
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Test","realm":"Test"},
        "sections":{"action_bar_observation":{"coverage":"loaded_builtin_buttons_only","observed_buttons":3,"missing_buttons":0,
            "slots":{"1":{"empty":false,"kind":"spell","id":123},"2":{"empty":true},"3":{"empty":false,"kind":"item","id":99}}}}
    }))?;
    let legacy: ActionRequest = serde_json::from_value(json!({"source_slot":1,"target_slot":2}))?;
    let moved = compile(&snapshot, &"a".repeat(64), &legacy)?;
    let request = ActionRequest {
        operation: ActionOperation::Copy,
        source_slot: 1,
        target_slot: 2,
    };
    let copied = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(copied["mode"], "copy");
    assert_eq!(copied["addon_execution_enabled"], false);
    assert_ne!(copied["operation_id"], moved["operation_id"]);
    let lua = Lua::new();
    let ao = lua.create_table()?;
    for source in [
        include_str!("../addon/ActionImport.lua"),
        include_str!("../addon/ActionOperations.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.globals().set("AO", ao.clone())?;
    lua.globals()
        .set("code", copied["import_code"].as_str().unwrap())?;
    lua.load(
        r#"
        AO.family='retail'; AO.variant='live'; AO.interface=120105
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end; function UnitChannelInfo() end
        slots={[1]={kind='spell',id=123}}; writes=0
        function GetActionInfo(slot) local a=slots[slot]; if a then return a.kind,a.id end end
        function GetCursorInfo() if cursor then return 'spell',0,nil,cursor.id end end
        AO.collectors={action_bar_observation=function() return {slots={['1']={},['2']={}}} end}
        C_Spell={PickupSpell=function(id) writes=writes+1; cursor={kind='spell',id=id} end}
        function PlaceAction(slot) writes=writes+1; slots[slot],cursor=cursor,nil end
        local op=AO.DecodeActionOperation(code)
        assert(op.kind=='copy_action_slot' and op.target.empty)
        assert(AO.ApplyActionOperation(op).status=='applied')
        assert(writes==2 and slots[1].id==123 and slots[2].id==123 and not cursor)
    "#,
    )
    .exec()?;
    let decoder: mlua::Function = ao.get("DecodeActionOperation")?;
    let occupied = compile(
        &snapshot,
        &"a".repeat(64),
        &ActionRequest {
            operation: ActionOperation::Move,
            source_slot: 1,
            target_slot: 3,
        },
    )?;
    let forged =
        occupied["import_code"]
            .as_str()
            .unwrap()
            .replacen("AO-ACTION-1", "AO-ACTIONCOPY-1", 1);
    assert!(decoder.call::<mlua::Table>(forged).is_err());
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                operation: ActionOperation::Copy,
                source_slot: 1,
                target_slot: 3
            }
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ActionRequest>(
            json!({"operation":"duplicate","source_slot":1,"target_slot":2})
        )
        .is_err()
    );
    Ok(())
}
