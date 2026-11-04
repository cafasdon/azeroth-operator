use mlua::Lua;

#[test]
fn creation_waits_for_unique_populated_selected_build_and_preserves_bystanders()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105}
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Synthetic' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end; function UnitChannelInfo() end
        function GetCursorInfo() return cursor end; function StaticPopup_Visible() return popup end
        function finish()
            entries['11']={name='New',uses_shared_action_bars=false,import_string=current,serialization_version=2}
            if mode~='unselected' then selected=11 end
            if mode=='wrong_content' then entries['11'].import_string='Tk8=' end
            if mode=='wrong_shared' then entries['11'].uses_shared_action_bars=true end
            if mode=='extra' then entries['12']=copy(entries['11']) end
            if mode=='bystander' then entries['8'].name='Changed' end
            if mode=='active' then current='Tk8=' end
        end
        function reset()
            mode=nil; writes=0; starts=0; selected=8; current='QUJDRA=='; combat=false; casting=nil; cursor=nil; popup=nil; staged=false
            visible=true; inspect=false; busy=false; canCreate=true
            entries={['8']={name='Old',uses_shared_action_bars=true,import_string='RUZHSA==',serialization_version=2}}
            op={kind='create_talent_config',family='retail',variant='live',build='70688',interface=120105,character='Fixture',realm='Synthetic',name='New',expected_new_shared_action_bars=false,
                expected={spec_id=263,active_config_id=7,selected_config_id=8,serialization_version=2,active_import_string=current,entries=copy(entries)}}
            C_SpecializationInfo={GetSpecialization=function() return 1 end,GetSpecializationInfo=function() return 263 end}
            C_ClassTalents={GetActiveConfigID=function() return 7 end,GetLastSelectedSavedConfigID=function() return selected end,
                GetConfigIDsBySpecID=function() local ids={} for id in pairs(entries) do ids[#ids+1]=tonumber(id) end return ids end,
                CanCreateNewConfig=function() return canCreate end,CanEditTalents=function() return true end,
                RequestNewConfig=function(name)
                    assert(name=='New'); writes=writes+1
                    if checkReceipt then assert(AzerothOperatorOperations==op.id..':outcome_unknown:create_talent_config;') end
                    if mode=='throw_before' then error('before') end
                    if mode~='delay' and mode~='reject' then finish() end
                    if mode=='throw_after' then error('after') end
                    return mode~='false_effect' and mode~='reject'
                end}
            C_Traits={GetLoadoutSerializationVersion=function() return 2 end,ConfigHasStagedChanges=function() return staged end,
                GenerateImportString=function(id) return id==7 and current or entries[tostring(id)].import_string end,
                GetConfigInfo=function(id) local e=entries[tostring(id)]; return {name=e.name,usesSharedActionBars=e.uses_shared_action_bars} end}
            frame={IsShown=function() return visible end,IsInspecting=function() return inspect end,IsCommitInProgress=function() return busy end,
                GetConfigID=function() return 7 end,HasAnyPurchasedRanks=function() return true end,
                OnTraitConfigCreateStarted=function(_,purchased)
                    assert(purchased); starts=starts+1; if mode=='callback_error' then error('callback') end
                end}
            PlayerSpellsFrame={TalentsFrame=frame}
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    for source in [
        include_str!("../addon/TalentMetadata.lua"),
        include_str!("../addon/TalentCreate.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(r#"
        assert(AO.ApplyTalentCreate(op).status=='applied' and writes==1 and starts==1)
        assert(not pcall(AO.ApplyTalentCreate,op) and writes==1)
        reset(); mode='false_effect'; assert(AO.ApplyTalentCreate(op).status=='applied' and starts==0)
        for _,failure in ipairs({'delay','reject','unselected','wrong_content','wrong_shared','extra','bystander','active'}) do
            reset(); mode=failure; assert(AO.ApplyTalentCreate(op).status=='outcome_unknown' and writes==1,failure)
        end
        reset(); mode='delay'; assert(AO.ApplyTalentCreate(op).status=='outcome_unknown'); finish()
        assert(AO.VerifyTalentCreate(op) and writes==1 and starts==1)
        for _,failure in ipairs({'throw_before','throw_after','callback_error'}) do
            reset(); mode=failure; assert(not pcall(AO.ApplyTalentCreate,op) and writes==1,failure)
        end
        for _,failure in ipairs({'combat','casting','cursor','popup','hidden','inspect','busy','capacity','staged','stale','collision','callback','api'}) do
            reset()
            if failure=='combat' then combat=true elseif failure=='casting' then casting='spell' elseif failure=='cursor' then cursor='item'
            elseif failure=='popup' then popup=true elseif failure=='hidden' then visible=false elseif failure=='inspect' then inspect=true
            elseif failure=='busy' then busy=true elseif failure=='capacity' then canCreate=false elseif failure=='staged' then staged=true
            elseif failure=='stale' then current='Tk8=' elseif failure=='collision' then op.name='OLD'
            elseif failure=='callback' then frame.OnTraitConfigCreateStarted=nil elseif failure=='api' then C_ClassTalents.RequestNewConfig=nil end
            assert(not pcall(AO.ApplyTalentCreate,op) and writes==0,failure)
        end
        reset(); op.expected_new_shared_action_bars=nil; assert(not pcall(AO.ApplyTalentCreate,op) and writes==0)
        reset(); frame.autoLoadNewConfigID=99; assert(not pcall(AO.ApplyTalentCreate,op) and writes==0)
        reset(); frame.nextNewConfigRequiresPopulatedCheck=true; assert(not pcall(AO.ApplyTalentCreate,op) and writes==0)
        reset(); frame.areClassTalentCommitVisualsActive=true; assert(not pcall(AO.ApplyTalentCreate,op) and writes==0)
        reset(); function issecretvalue(v) return v==true end
        assert(not pcall(AO.ApplyTalentCreate,op) and writes==0); issecretvalue=nil
        reset(); AO.family='forever'; assert(not pcall(AO.ApplyTalentCreate,op) and writes==0)
    "#).exec()?;
    let mut snapshot: azeroth_operator::Snapshot = serde_json::from_value(
        serde_json::json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-04T00:00:00Z","client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Fixture","realm":"Synthetic"},"sections":{"talents":{"config_id":7,"nodes":{},"loadout_export":{"status":"available","format":"blizzard_traits","import_string":"QUJDRA==","spec_id":263,"serialization_version":2,"staged_changes":false},"saved_loadouts":{"spec_id":263,"selected_config_id":8,"entries":{"8":{"name":"Old","uses_shared_action_bars":true,"import_string":"RUZHSA==","serialization_version":2}}}}}}),
    )?;
    let request = serde_json::json!({"operation":"create","name":"New","expected_new_shared_action_bars":false});
    let compile = |s: &azeroth_operator::Snapshot, r: serde_json::Value| {
        azeroth_operator::talent_operation::compile(s, &"a".repeat(64), &serde_json::from_value(r)?)
    };
    let prepared = compile(&snapshot, request.clone())?;
    assert_eq!(prepared["execution_supported"], true);
    assert_eq!(prepared["requires_addon_version"], "0.15.0-dev");
    assert_eq!(prepared["selects_new_build"], true);
    assert!(prepared.get("target_config_id").is_none());
    let code = prepared["import_code"].as_str().unwrap();
    assert_eq!(code.split(':').count(), 24);
    lua.load(include_str!("../addon/TalentCreateImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeTalentCreate")?;
    let decoded: mlua::Table = decoder.call(code)?;
    assert_eq!(
        decoded.get::<String>("id")?,
        prepared["operation_id"].as_str().unwrap()
    );
    assert!(!decoded.get::<bool>("expected_new_shared_action_bars")?);
    lua.load("reset(); AO.family='retail'").exec()?;
    lua.globals().set("op", decoded)?;
    lua.load("assert(AO.ApplyTalentCreate(op).status=='applied' and writes==1)")
        .exec()?;
    lua.globals().set("code", code)?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(r#"
        function GetTime() return now end
        function setup()
            reset(); AO.family='retail'; AzerothOperatorOperations=''; now=0; checkReceipt=true
            op=AO.DecodeTalentCreate(code)
        end
        setup(); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code..':-') and writes==0)
        AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
        assert(ok and r.status=='applied' and writes==1)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        SlashCmdList={}; function GetServerTime() return 0 end
        function date() return '2026-11-04T00:00:00Z' end
        AO.collectors={operation_receipts=function() return AO.ReadMacroReceipts() end}
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.Capture()").exec()?;
    let receipt: String = lua.globals().get("AzerothOperatorOperations")?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let imported = azeroth_operator::import::decode(
        format!("AzerothOperatorExport = \"{export}\"\nAzerothOperatorOperations = \"{receipt}\"")
            .as_bytes(),
    )?;
    assert_eq!(
        imported.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()],
        serde_json::json!({"kind":"create_talent_config","status":"applied"})
    );
    lua.load(r#"
        setup(); mode='delay'; AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
        assert(ok and r.status=='outcome_unknown' and writes==1)
        finish(); assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==1)
        setup(); mode='delay'; AO.ReviewMacroImport(code); AO.ApplyReviewedMacro(code)
        now=61; finish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==1)
        for _,failure in ipairs({'throw_before','throw_after','callback_error'}) do
            setup(); mode=failure; AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
            assert(not ok and r.status=='outcome_unknown' and writes==1)
            assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
            AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        end
        setup(); mode='delay'; AO.ReviewMacroImport(code); AO.ApplyReviewedMacro(code)
    "#).exec()?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("finish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown'); AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)").exec()?;
    for (index, value) in [
        (0, "AO-TALMETA-1"),
        (3, "classic"),
        (10, "9"),
        (11, "7c"),
        (12, "0263"),
        (17, "128"),
        (18, "2"),
        (19, "7"),
        (21, "2"),
        (22, "3"),
        (23, "-"),
    ] {
        let mut fields: Vec<_> = code.split(':').collect();
        fields[index] = value;
        assert!(decoder.call::<mlua::Table>(fields.join(":")).is_err());
    }
    assert!(
        decoder
            .call::<mlua::Table>(format!("{code}:extra"))
            .is_err()
    );
    let mut duplicate: Vec<_> = code.split(':').collect();
    duplicate[17] = "2";
    duplicate.extend(duplicate[19..24].to_vec());
    assert!(decoder.call::<mlua::Table>(duplicate.join(":")).is_err());
    for bad in [
        serde_json::json!({"operation":"create","name":"New"}),
        serde_json::json!({"operation":"create","name":"OLD","expected_new_shared_action_bars":false}),
        serde_json::json!({"operation":"create","target_config_id":8,"name":"New","expected_new_shared_action_bars":false}),
        serde_json::json!({"operation":"create","name":" bad","expected_new_shared_action_bars":false}),
        serde_json::json!({"target_config_id":8,"expected_new_shared_action_bars":false}),
    ] {
        assert!(compile(&snapshot, bad).is_err());
    }
    let shared = compile(
        &snapshot,
        serde_json::json!({"operation":"create","name":"New","expected_new_shared_action_bars":true}),
    )?;
    assert_ne!(shared["operation_id"], prepared["operation_id"]);
    snapshot.sections.get_mut("talents").unwrap()["saved_loadouts"] =
        serde_json::json!({"spec_id":263,"entries":{}});
    let empty = compile(&snapshot, request.clone())?;
    assert_eq!(
        empty["import_code"].as_str().unwrap().split(':').count(),
        19
    );
    let empty_decoded: mlua::Table = decoder.call(empty["import_code"].as_str().unwrap())?;
    let expected: mlua::Table = empty_decoded.get("expected")?;
    assert!(expected.get::<Option<u32>>("selected_config_id")?.is_none());
    for id in 10..138 {
        snapshot.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]
            [id.to_string()] = serde_json::json!({"name":format!("Build{id}"),"uses_shared_action_bars":false,"import_string":"QUJDRA==","serialization_version":2});
    }
    assert!(compile(&snapshot, request).is_err());
    Ok(())
}
