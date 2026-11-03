use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn lua_export_roundtrip_and_failed_capture_preserves_previous()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO = {}; SlashCmdList = {}; combat = false
        function InCombatLockdown() return combat end
        function GetBuildInfo() return "12.1.5", "70688", "", 120105 end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink(_,slot) if slot==16 then return 'quoted "item" \\ link' end end
        function UnitLevel() return 80 end
        function UnitClass() return "Shaman", "SHAMAN" end
        function UnitRace() return "Goblin", "Goblin" end
        function UnitName() return "Example" end
        function GetRealmName() return "Test" end
        function GetServerTime() return 1735689600 end
        function date() return "2025-01-01T00:00:00Z" end
        C_Container = {}
        function C_Container.GetContainerNumSlots() return 1 end
        function C_Container.GetContainerItemInfo(bag,slot)
            if bag==0 then return {itemID=456,stackCount=2,hyperlink="test"} end
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../clients/retail/Adapter.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    assert_eq!(snapshot.sections["bags"]["0"]["1"]["count"], 2);
    assert_eq!(snapshot.unavailable["talents"], "api_unavailable_or_failed");
    assert_eq!(
        snapshot.sections["equipment"]["16"]["link"],
        "quoted \"item\" \\ link"
    );
    lua.load("combat=true; assert(not pcall(AO.Capture))")
        .exec()?;
    assert_eq!(hex, lua.globals().get::<String>("AzerothOperatorExport")?);
    lua.load(r#"
        combat=false
        MAX_ACCOUNT_MACROS=120; MAX_ACTION_BUTTONS=180
        function GetNumMacros() return 1,1 end
        function GetMacroInfo(index) return "Macro"..index,123,"/cast Test" end
        function GetNumBindings() return 1 end
        function GetBinding() return "ACTIONBUTTON1","Action Bar","1",nil end
        function GetActionInfo(slot) if slot==1 then return "spell",123,nil end end
        C_ClassTalents={GetActiveConfigID=function() return 7 end,HasUnspentTalentPoints=function() return true,1,2 end}
        C_Traits={GetConfigInfo=function() return {name="Build",treeIDs={5}} end,
          GetTreeNodes=function() return {10} end,
          GetNodeInfo=function() return {activeEntry={entryID=11,rank=2}} end}
        function GetProfessions() return 1,nil,3 end
        function GetProfessionInfo(i) return "Profession"..i,0,10,100,0,0,200+i end
        C_QuestLog={GetNumQuestLogEntries=function() return 1 end,GetInfo=function() return {questID=42,title="Test quest",level=80} end}
        C_CurrencyInfo={GetCurrencyListSize=function() return 1 end,GetCurrencyListInfo=function() return {name="Currency",quantity=5} end}
        function GetTotalAchievementPoints() return 1234 end
        AO.Capture()
    "#).exec()?;
    let full_hex: String = lua.globals().get("AzerothOperatorExport")?;
    let full = decode(format!("AzerothOperatorExport = \"{full_hex}\"").as_bytes())?;
    assert!(full.unavailable.is_empty());
    assert_eq!(full.sections.len(), 11);
    assert_eq!(full.sections["macros"]["121"]["scope"], "character");
    assert_eq!(full.sections["bindings"]["1"]["key1"], "1");
    assert_eq!(full.sections["action_bars"]["1"]["id"], 123);
    assert_eq!(full.sections["talents"]["nodes"]["10"]["rank"], 2);
    assert_eq!(full.sections["professions"]["3"]["rank"], 10);
    assert_eq!(full.sections["quests"]["42"]["title"], "Test quest");
    assert_eq!(
        full.sections["currencies"]["coverage"],
        "visible_currency_list_only"
    );
    assert_eq!(full.sections["achievements"]["total_points"], 1234);
    // Mutate the actual producer output, rather than validating hand-built happy paths.
    for name in full.sections.keys() {
        let mut malformed = full.clone();
        malformed
            .sections
            .insert(name.clone(), serde_json::json!([]));
        assert!(
            decode(&serde_json::to_vec(&malformed)?).is_err(),
            "{name} accepted an array"
        );
        let mut malformed = full.clone();
        malformed.sections.get_mut(name).unwrap()["unexpected_field"] = serde_json::json!(true);
        assert!(
            decode(&serde_json::to_vec(&malformed)?).is_err(),
            "{name} accepted an unknown field"
        );
    }
    for (section, pointer) in [
        ("talents", "/nodes/10/rank"),
        ("currencies", "/entries/1/quantity"),
        ("professions", "/3/rank"),
        ("achievements", "/total_points"),
        ("stats", "/level"),
        ("action_bars", "/1/id"),
    ] {
        let mut malformed = full.clone();
        *malformed
            .sections
            .get_mut(section)
            .unwrap()
            .pointer_mut(pointer)
            .unwrap() = serde_json::json!("wrong type");
        assert!(
            decode(&serde_json::to_vec(&malformed)?).is_err(),
            "{section}{pointer} accepted wrong type"
        );
    }
    assert_eq!(
        full.sections["talents"]["loadout_export"]["reason"],
        "api_unavailable"
    );
    lua.load(r#"
        export_calls=0
        C_SpecializationInfo={GetSpecialization=function() return 2 end,GetSpecializationInfo=function() return 263 end}
        C_Traits.ConfigHasStagedChanges=function() return false end
        C_Traits.GetLoadoutSerializationVersion=function() return 2 end
        C_Traits.GenerateImportString=function(id) assert(id==7); export_calls=export_calls+1; return "QUJDRA==" end
        AO.Capture()
    "#).exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let loadout = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert_eq!(
        loadout.sections["talents"]["loadout_export"]["import_string"],
        "QUJDRA=="
    );
    assert_eq!(
        loadout.sections["talents"]["loadout_export"]["spec_id"],
        263
    );
    for (setup, reason) in [
        (
            "C_Traits.ConfigHasStagedChanges=function() return true end",
            "staged_changes",
        ),
        (
            "C_Traits.ConfigHasStagedChanges=function() return false end; AO.family='forever'",
            "client_not_supported",
        ),
        (
            "AO.family='retail'; issecretvalue=function(v) return v=='QUJDRA==' end",
            "restricted_or_failed",
        ),
        (
            "issecretvalue=nil; C_Traits.GenerateImportString=function() error('API error') end",
            "restricted_or_failed",
        ),
    ] {
        lua.load(setup).exec()?;
        lua.load("AO.Capture()").exec()?;
        let hex: String = lua.globals().get("AzerothOperatorExport")?;
        let snapshot = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
        assert_eq!(
            snapshot.sections["talents"]["loadout_export"]["reason"],
            reason
        );
        assert_eq!(snapshot.sections["talents"]["nodes"]["10"]["rank"], 2);
        assert!(
            snapshot.sections["talents"]["loadout_export"]
                .get("import_string")
                .is_none()
        );
    }
    let last_hex: String = lua.globals().get("AzerothOperatorExport")?;
    lua.load("AO.interface=1; assert(not pcall(AO.Capture))")
        .exec()?;
    assert_eq!(
        last_hex,
        lua.globals().get::<String>("AzerothOperatorExport")?
    );
    Ok(())
}
