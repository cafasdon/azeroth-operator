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
        target_config_id: Some(9),
        ..Default::default()
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(prepared["expected_active_config_id"], 7);
    assert_eq!(prepared["expected_selected_config_id"], 8);
    assert_eq!(prepared["desired"]["import_string"], "RUZHSA==");
    let rename: TalentRequest = serde_json::from_value(
        json!({"operation":"rename","target_config_id":9,"name":"New build"}),
    )?;
    let draft = compile(&snapshot, &"a".repeat(64), &rename)?;
    assert_eq!(draft["kind"], "rename_talent_config");
    assert_eq!(
        draft["expected"]["entries"],
        snapshot.sections["talents"]["saved_loadouts"]["entries"]
    );
    assert_eq!(draft["expected"]["active_import_string"], "QUJDRA==");
    assert_eq!(draft["execution_supported"], true);
    let metadata_code = draft["import_code"].as_str().unwrap();
    let metadata_lua = mlua::Lua::new();
    let metadata_ao = metadata_lua.create_table().unwrap();
    metadata_lua
        .load(include_str!("../addon/TalentMetadataImport.lua"))
        .call::<()>(("AzerothOperator", metadata_ao.clone()))
        .unwrap();
    let metadata_decoder: mlua::Function = metadata_ao.get("DecodeTalentMetadata").unwrap();
    let decoded: mlua::Table = metadata_decoder.call(metadata_code).unwrap();
    assert_eq!(decoded.get::<String>("name").unwrap(), "New build");
    assert_eq!(
        decoded.get::<String>("id").unwrap(),
        draft["operation_id"].as_str().unwrap()
    );
    let expected: mlua::Table = decoded.get("expected").unwrap();
    assert_eq!(expected.get::<u32>("active_config_id").unwrap(), 7);
    assert_eq!(expected.get::<u32>("selected_config_id").unwrap(), 8);
    assert_eq!(
        expected.get::<String>("active_import_string").unwrap(),
        "QUJDRA=="
    );
    let captured: mlua::Table = expected.get("entries").unwrap();
    let entry: mlua::Table = captured.get("9").unwrap();
    assert_eq!(entry.get::<String>("import_string").unwrap(), "RUZHSA==");
    assert!(!entry.get::<bool>("uses_shared_action_bars").unwrap());
    for (index, bad) in [
        (0, "AO-TALMETA-2"),
        (1, "bad"),
        (3, "classic"),
        (9, "load_talent_config"),
        (10, "7"),
        (11, "7c"),
        (12, "0263"),
        (14, "-1"),
        (16, "7c"),
        (17, "2"),
        (18, "7"),
        (20, "2"),
        (21, "3"),
        (22, "-"),
    ] {
        let mut fields: Vec<_> = metadata_code.split(':').collect();
        fields[index] = bad;
        assert!(
            metadata_decoder
                .call::<mlua::Table>(fields.join(":"))
                .is_err(),
            "accepted malformed field {index}"
        );
    }
    assert!(
        metadata_decoder
            .call::<mlua::Table>(format!("{metadata_code}:extra"))
            .is_err()
    );
    assert!(
        metadata_decoder
            .call::<mlua::Table>("a".repeat(524289))
            .is_err()
    );
    assert_eq!(draft, compile(&snapshot, &"a".repeat(64), &rename)?);
    let delete: TalentRequest =
        serde_json::from_value(json!({"operation":"delete","target_config_id":9}))?;
    let delete_draft = compile(&snapshot, &"a".repeat(64), &delete)?;
    let delete_code = delete_draft["import_code"].as_str().unwrap();
    let decoded_delete: mlua::Table = metadata_decoder.call(delete_code).unwrap();
    assert!(
        decoded_delete
            .get::<Option<String>>("name")
            .unwrap()
            .is_none()
    );
    let mut selected_delete: Vec<_> = delete_code.split(':').collect();
    selected_delete[14] = "9";
    assert!(
        metadata_decoder
            .call::<mlua::Table>(selected_delete.join(":"))
            .is_ok()
    );
    let mut duplicate: Vec<_> = metadata_code.split(':').collect();
    duplicate[17] = "2";
    let record = duplicate[18..23].to_vec();
    duplicate.extend(record);
    assert!(
        metadata_decoder
            .call::<mlua::Table>(duplicate.join(":"))
            .is_err()
    );
    assert_eq!(
        compile(&snapshot, &"a".repeat(64), &delete)?["kind"],
        "delete_talent_config"
    );
    let mut changed = snapshot.clone();
    changed.sections.get_mut("talents").unwrap()["saved_loadouts"]["selected_config_id"] = json!(9);
    let selected = compile(&changed, &"a".repeat(64), &delete)?;
    assert_eq!(selected["clears_saved_selection"], true);
    assert_eq!(selected["execution_supported"], true);
    assert_eq!(selected["requires_addon_version"], "0.16.0-dev");
    assert!(compile(&changed, &"a".repeat(64), &rename).is_ok());
    changed.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]["10"] = json!({"name":"NEW BUILD","import_string":"QUJDRA==","serialization_version":2,"uses_shared_action_bars":true});
    assert!(compile(&changed, &"a".repeat(64), &rename).is_err());
    changed.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]["10"]["name"] =
        json!("Bystander");
    assert_ne!(
        compile(&changed, &"a".repeat(64), &rename)?["operation_id"],
        draft["operation_id"]
    );
    changed.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]["10"]["serialization_version"] =
        json!(3);
    assert!(compile(&changed, &"a".repeat(64), &rename).is_err());
    for value in [
        json!({"operation":"rename","target_config_id":9}),
        json!({"operation":"delete","target_config_id":9,"name":"x"}),
        json!({"target_config_id":9,"name":"x"}),
        json!({"operation":"rename","target_config_id":9,"name":" bad"}),
        json!({"operation":"rename","target_config_id":9,"name":"a|b"}),
    ] {
        assert!(compile(&snapshot, &"a".repeat(64), &serde_json::from_value(value)?).is_err());
    }
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
