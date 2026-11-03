use azeroth_operator::{
    Snapshot,
    talent_operation::{TalentRequest, compile},
};
use serde_json::json;
#[test]
fn saved_talent_preparation_binds_contents_and_refuses_uncertainty() -> anyhow::Result<()> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z","client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},"sections":{"talents":{"config_id":7,"nodes":{},"loadout_export":{"status":"available","format":"blizzard_traits","import_string":"QUJDRA==","spec_id":263,"serialization_version":2,"staged_changes":false},"saved_loadouts":{"spec_id":263,"selected_config_id":8,"entries":{"9":{"name":"Build","import_string":"RUZHSA==","serialization_version":2,"uses_shared_action_bars":false}}}}}}),
    )?;
    let request = TalentRequest {
        target_config_id: 9,
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(prepared["expected_active_config_id"], 7);
    assert_eq!(prepared["expected_selected_config_id"], 8);
    assert_eq!(prepared["desired"]["import_string"], "RUZHSA==");
    assert_eq!(
        prepared["import_code"].as_str().unwrap().split(':').count(),
        18
    );
    let lua = mlua::Lua::new();
    let ao = lua.create_table().unwrap();
    lua.load(include_str!("../addon/TalentImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))
        .unwrap();
    let decoder: mlua::Function = ao.get("DecodeTalentOperation").unwrap();
    let code = prepared["import_code"].as_str().unwrap();
    let decoded: mlua::Table = decoder.call(code).unwrap();
    assert_eq!(
        decoded.get::<String>("expected_import_string").unwrap(),
        "QUJDRA=="
    );
    assert_eq!(
        decoded.get::<String>("desired_import_string").unwrap(),
        "RUZHSA=="
    );
    assert_eq!(
        decoded.get::<u32>("expected_selected_config_id").unwrap(),
        8
    );
    assert!(!decoded.get::<bool>("uses_shared_action_bars").unwrap());
    lua.globals().set("AO", ao.clone()).unwrap();
    lua.globals().set("code", code).unwrap();
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao))
        .unwrap();
    lua.load(r#"
        AO.family='retail'; AO.variant='live'; now=0; writes=0; verified=false
        function GetTime() return now end
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function AO.ApplyTalentOperation(op) writes=writes+1; return {kind=op.kind,status='outcome_unknown'} end
        function AO.VerifyTalentOperation() return verified end
        AO.ReviewMacroImport(code)
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='outcome_unknown' and writes==1)
        local id=AO.DecodeTalentOperation(code).id
        assert(AO.ReadMacroReceipts()[id].status=='outcome_unknown')
        verified=true; now=10
        assert(AO.ReadMacroReceipts()[id].status=='applied' and writes==1)
        local late=code:gsub('AO%-TALENT%-1:[0-9a-f]+:','AO-TALENT-1:'..string.rep('f',64)..':',1)
        verified=false; AO.ReviewMacroImport(late); AO.ApplyReviewedMacro(late)
        now=71; verified=true
        assert(AO.ReadMacroReceipts()[string.rep('f',64)].status=='outcome_unknown' and writes==2)
        AO.ReviewMacroImport(late); assert(not pcall(AO.ApplyReviewedMacro,late) and writes==2)
    "#).exec().unwrap();
    for (field, bad) in [(14, "7c"), (16, "2"), (9, "0263"), (17, "-1")] {
        let mut fields: Vec<_> = code.split(':').collect();
        fields[field] = bad;
        assert!(decoder.call::<mlua::Table>(fields.join(":")).is_err());
    }
    snapshot.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]["9"]["import_string"] =
        json!("SUpLTA==");
    assert_ne!(
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot.sections.get_mut("talents").unwrap()["saved_loadouts"]["spec_id"] = json!(264);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    snapshot.sections.get_mut("talents").unwrap()["saved_loadouts"]["spec_id"] = json!(263);
    snapshot.sections.get_mut("talents").unwrap()["loadout_export"] =
        json!({"status":"unavailable","reason":"staged_changes"});
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}
