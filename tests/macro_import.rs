use azeroth_operator::{
    Snapshot,
    macro_operation::{MacroRequest, compile},
};
use mlua::{Lua, Table};
use serde_json::json;

#[test]
fn compiler_binds_existing_macro_contents_and_scope() -> anyhow::Result<()> {
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Test","realm":"Test Realm"},
        "sections":{"macros":{"1":{"name":"Existing","icon":134400,"body":"/say original","scope":"account"}}}
    }))?;
    for request in [
        json!({"kind":"edit","source_index":1,"desired":{"name":"Edited","icon":134400,"body":""}}),
        json!({"kind":"delete","source_index":1}),
        json!({"kind":"move","source_index":1,"target_scope":"character"}),
    ] {
        let operation = compile(
            &snapshot,
            &"b".repeat(64),
            &serde_json::from_value(request)?,
        )?;
        assert_eq!(operation["expected"]["body"], "/say original");
        assert_eq!(operation["source_scope"], "account");
        let lua = Lua::new();
        let ao = lua.create_table().unwrap();
        lua.load(include_str!("../addon/MacroImport.lua"))
            .call::<()>(("AzerothOperator", ao.clone()))
            .unwrap();
        let decode: mlua::Function = ao.get("DecodeMacroOperation").unwrap();
        let decoded: Table = decode
            .call(operation["import_code"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            decoded.get::<String>("kind").unwrap(),
            operation["kind"].as_str().unwrap()
        );
    }
    for request in [
        json!({"kind":"delete","source_index":2}),
        json!({"kind":"move","source_index":1,"target_scope":"account"}),
        json!({"kind":"edit","source_index":1,"desired":{"name":"Too long macro name","icon":134400,"body":""}}),
    ] {
        assert!(
            compile(
                &snapshot,
                &"b".repeat(64),
                &serde_json::from_value(request)?
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn compiled_operation_applies_once_and_rejects_wrong_target() -> anyhow::Result<()> {
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Test","realm":"Test Realm"},"sections":{"macros":{}}
    }))?;
    let request: MacroRequest =
        serde_json::from_value(json!({"kind":"create","target_scope":"account",
        "desired":{"name":"AO Test","icon":134400,"body":"/say a|b\n/say olá"}}))?;
    let compiled = compile(&snapshot, &"a".repeat(64), &request)?;
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105}
        who='Wrong'; writes=0; macros={}
        Constants={MacroConsts={MAX_ACCOUNT_MACROS=120,MAX_CHARACTER_MACROS=30}}
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return who end
        function GetRealmName() return 'Test Realm' end
        function InCombatLockdown() return false end
        function GetNumMacros() return #macros,0 end
        function GetMacroInfo(i) local m=macros[i]; if m then return m.name,m.icon,m.body end end
        function CreateMacro(n,i,b) writes=writes+1; macros[#macros+1]={name=n,icon=i,body=b}; return #macros end
    "#).exec().unwrap();
    let ao: Table = lua.globals().get("AO").unwrap();
    for source in [
        include_str!("../addon/MacroOperations.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))
            .unwrap();
    }
    lua.globals()
        .set("code", compiled["import_code"].as_str().unwrap())
        .unwrap();
    lua.load(
        r#"
        local decoded=AO.DecodeMacroOperation(code)
        assert(decoded.desired.body=='/say a|b\n/say olá')
        assert(not pcall(AO.ApplyImportedMacro,code))
        assert(writes==0 and AzerothOperatorOperations==nil)
        who='Test'
        local receipt,ok=AO.ApplyImportedMacro(code)
        assert(ok and receipt.status=='applied' and writes==1)
        assert(not pcall(AO.ApplyImportedMacro,code))
        assert(writes==1)
        assert(type(AzerothOperatorOperations)=='string')
        local checkpoint=AzerothOperatorOperations
        local receipts,count=AO.ReadMacroReceipts()
        assert(count==1 and receipts[decoded.id].status=='applied')
        AzerothOperatorOperations=checkpoint..checkpoint
        assert(not pcall(AO.ReadMacroReceipts))
        AzerothOperatorOperations=checkpoint
        assert(not pcall(AO.DecodeMacroOperation,code..'|extra'))
        assert(not pcall(AO.DecodeMacroOperation,'return os.execute("bad")'))
    "#,
    )
    .exec()
    .unwrap();
    Ok(())
}
