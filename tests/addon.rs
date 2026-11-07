use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn lua_export_roundtrip_and_failed_capture_preserves_previous()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO = {}; SlashCmdList = {}; combat = false
        function GetCurrentBindingSet() return 2 end
        C_KeyBindings={GetBindingContextForAction=function() return 0 end}
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end,
            CreateFromEquipmentSlot=function(_,slot) return {slot=slot} end}
        C_Item={GetItemGUID=function(location) return location.bag~=nil and 'Item-Bag-Instance' or 'Item-Equipment-Instance' end}
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
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    lua.globals().set(
        "AzerothOperatorOperations",
        format!("{}:outcome_unknown:move;", "a".repeat(64)),
    )?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    assert_eq!(
        snapshot.sections["equipment"]["16"]["guid"],
        "Item-Equipment-Instance"
    );
    assert_eq!(
        snapshot.sections["bags"]["0"]["1"]["guid"],
        "Item-Bag-Instance"
    );
    assert_eq!(snapshot.sections["bags"]["0"]["1"]["count"], 2);
    lua.load("local original=C_Item.GetItemGUID; C_Item.GetItemGUID=function() error('unavailable') end; assert(AO.collectors.equipment()['16'].guid==nil); assert(AO.collectors.bags()['0']['1'].guid==nil); C_Item.GetItemGUID=original").exec()?;
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
        NUM_ACTIONBAR_BUTTONS=2; ActionButtonUtil={ActionBarButtonNames={'ActionButton'}}
        ActionButton1={action=1}; ActionButton2={action=2}
        function GetNumMacros() return 1,1 end
        function GetMacroInfo(index) return "Macro"..index,123,"/cast Test" end
        function GetNumBindings() return 1 end
        function GetBinding() return "ACTIONBUTTON1","Action Bar","1",nil end
        function GetActionInfo(slot) if slot==1 then return "spell",123,nil end end
        C_ClassTalents={GetActiveConfigID=function() return 7 end,HasUnspentTalentPoints=function() return true,1,2 end}
        C_Traits={GetConfigInfo=function() return {name="Build",treeIDs={5},usesSharedActionBars=true} end,
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
    assert_eq!(full.unavailable.len(), 1);
    assert!(full.unavailable.contains_key("equipment_sets"));
    assert_eq!(full.sections.len(), 14);
    assert_eq!(full.sections["binding_state"]["binding_set"], 2);
    assert_eq!(full.sections["bindings"]["1"]["context"], 0);
    assert_eq!(
        full.sections["operation_receipts"]["a".repeat(64)]["status"],
        "outcome_unknown"
    );
    assert_eq!(full.sections["macros"]["121"]["scope"], "character");
    assert_eq!(full.sections["bindings"]["1"]["key1"], "1");
    assert_eq!(full.sections["action_bars"]["1"]["id"], 123);
    assert_eq!(
        full.sections["action_bar_observation"]["coverage"],
        "loaded_builtin_buttons_only"
    );
    assert_eq!(
        full.sections["action_bar_observation"]["slots"]["2"]["empty"],
        true
    );
    for (pointer, value) in [
        ("/coverage", serde_json::json!("complete")),
        ("/observed_buttons", serde_json::json!(1)),
        ("/missing_buttons", serde_json::json!(2048)),
        ("/slots/1/empty", serde_json::json!(true)),
        ("/slots/1/kind", serde_json::json!("")),
    ] {
        let mut malformed = full.clone();
        *malformed
            .sections
            .get_mut("action_bar_observation")
            .unwrap()
            .pointer_mut(pointer)
            .unwrap() = value;
        assert!(malformed.validate().is_err());
    }
    lua.load(r#"
        MAX_ACTION_BUTTONS=nil; ActionButton2=nil
        local partial=AO.collectors.action_bar_observation()
        assert(partial.observed_buttons==1 and partial.missing_buttons==1 and partial.slots['2']==nil)
        ActionButton2={action=1}
        local duplicated=AO.collectors.action_bar_observation()
        assert(duplicated.observed_buttons==2 and duplicated.slots['1'] and duplicated.slots['2']==nil)
        ActionButton2={action=2}
        issecretvalue=function(value) return value==123 end
        assert(not pcall(AO.collectors.action_bar_observation)); issecretvalue=nil
        AO.family='forever'; assert(not pcall(AO.collectors.action_bar_observation)); AO.family='retail'
        ActionButton1=nil; ActionButton2=nil
        assert(not pcall(AO.collectors.action_bar_observation))
        ActionButton1={action=1}; ActionButton2={action=2}; MAX_ACTION_BUTTONS=180
    "#).exec()?;

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
    lua.load(
        r#"
        C_ClassTalents.GetConfigIDsBySpecID=function(spec) assert(spec==263); return {8,9} end
        C_ClassTalents.GetLastSelectedSavedConfigID=function() return 8 end
        C_Traits.GenerateImportString=function(id) return id==8 and 'QUJDRA==' or 'RUZHSA==' end
        AO.Capture()
    "#,
    )
    .exec()?;
    let captured: String = lua.globals().get("AzerothOperatorExport")?;
    let catalog = decode(format!("AzerothOperatorExport = \"{captured}\"").as_bytes())?;
    assert_eq!(
        catalog.sections["talents"]["saved_loadouts"]["selected_config_id"],
        8
    );
    assert_eq!(
        catalog.sections["talents"]["saved_loadouts"]["entries"]["9"]["import_string"],
        "RUZHSA=="
    );
    assert_eq!(
        catalog.sections["talents"]["saved_loadouts"]["entries"]["9"]["uses_shared_action_bars"],
        true
    );
    let mut malformed = catalog.clone();
    malformed.sections.get_mut("talents").unwrap()["saved_loadouts"]["entries"]["9"]["import_string"] =
        serde_json::json!("bad|code");
    assert!(malformed.validate().is_err());
    lua.load("C_ClassTalents.GetConfigIDsBySpecID=function() return {8,8} end; assert(AO.collectors.talents().saved_loadouts==nil); C_ClassTalents.GetConfigIDsBySpecID=nil; C_Traits.GenerateImportString=function() return 'QUJDRA==' end").exec()?;
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
    // A failed primary collector must not discard unrelated character data.
    for (name, setup, restore) in [
        (
            "equipment",
            "saved=GetInventoryItemID; GetInventoryItemID=function() error('restricted') end",
            "GetInventoryItemID=saved",
        ),
        (
            "bags",
            "saved=C_Container.GetContainerItemInfo; C_Container.GetContainerItemInfo=function() error('restricted') end",
            "C_Container.GetContainerItemInfo=saved",
        ),
        (
            "stats",
            "saved=UnitLevel; UnitLevel=function() error('restricted') end",
            "UnitLevel=saved",
        ),
    ] {
        lua.load(setup).exec()?;
        lua.load("AO.Capture()").exec()?;
        let hex: String = lua.globals().get("AzerothOperatorExport")?;
        let partial = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
        assert!(!partial.sections.contains_key(name));
        assert_eq!(partial.unavailable[name], "api_unavailable_or_failed");
        assert_eq!(partial.sections["quests"]["42"]["title"], "Test quest");
        assert_eq!(partial.sections.len(), 13);
        lua.load(restore).exec()?;
    }
    lua.load("saved=C_Container; C_Container=nil; AO.Capture()")
        .exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let missing = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert!(!missing.sections.contains_key("bags"));
    assert_eq!(missing.unavailable["bags"], "api_unavailable_or_failed");
    lua.load("C_Container=saved; AO.Capture()").exec()?;
    // Encodings must be capture-local: reusing a collector table cannot return stale data.
    lua.load(
        r#"
        saved_collector=AO.collectors.equipment
        reused={['16']={item_id=701}}
        AO.collectors.equipment=function() return reused end
        AO.Capture()
        reused['16'].item_id=702
        AO.Capture()
    "#,
    )
    .exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let changed = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(changed.sections["equipment"]["16"]["item_id"], 702);
    lua.load("reused['16'].link=function() end; AO.Capture()")
        .exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let invalid = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert!(!invalid.sections.contains_key("equipment"));
    assert_eq!(
        invalid.unavailable["equipment"],
        "restricted_or_unserializable"
    );
    lua.load("AO.collectors.equipment=saved_collector; AO.Capture()")
        .exec()?;
    // Hex lookup preserves control-byte escaping and multibyte UTF-8 payloads.
    lua.load(
        r#"
        local bytes={}
        for byte=0,127 do bytes[#bytes+1]=string.char(byte) end
        encoding_body=table.concat(bytes)..'ação 世界 😀'
        saved_macro=AO.collectors.macros
        AO.collectors.macros=function()
            return {['1']={name='Encoding',body=encoding_body,scope='account'}}
        end
        AO.Capture()
    "#,
    )
    .exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(
        hex.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    let encoded = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    let expected: String = (0u8..128).map(char::from).collect();
    assert_eq!(
        encoded.sections["macros"]["1"]["body"],
        expected + "ação 世界 😀"
    );
    lua.load("AO.collectors.macros=saved_macro; AO.Capture()")
        .exec()?;
    lua.load(
        "Constants={MacroConsts={MAX_ACCOUNT_MACROS=120}}; MAX_ACCOUNT_MACROS=nil; AO.Capture()",
    )
    .exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let namespaced = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(namespaced.sections["macros"]["121"]["scope"], "character");
    assert!(!namespaced.unavailable.contains_key("macros"));
    let last_hex: String = lua.globals().get("AzerothOperatorExport")?;
    lua.load("AO.interface=1; assert(not pcall(AO.Capture))")
        .exec()?;
    assert_eq!(
        last_hex,
        lua.globals().get::<String>("AzerothOperatorExport")?
    );
    Ok(())
}

#[test]
fn forever_bank_export_distinguishes_observed_empty_and_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live',interface=16001};SlashCmdList={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Example' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID() end
        NUM_TOTAL_EQUIPPED_BAG_SLOTS=4
        Enum={BankType={Character=0,Account=1}}
        BankFrame={IsShown=function() return bankOpen end}
        C_Bank={
            CanViewBank=function(kind) return kind==0 or accountVisible end,
            FetchNumPurchasedBankTabs=function(kind) return kind==0 and 1 or 0 end,
            FetchPurchasedBankTabData=function(kind)
                return kind==0 and {{ID=6,bankType=0}} or {}
            end
        }
        C_Container={
            GetContainerNumSlots=function(bag) return bag==6 and 2 or 0 end,
            GetContainerNumFreeSlots=function() return freeSlots end,
            GetContainerItemInfo=function(bag,slot)
                if bag==6 and slot==1 then return {itemID=744,stackCount=malformed and 0 or 1,hyperlink='item:744:7'} end
            end
        }
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end}
        C_Item={GetItemGUID=function() return 'Item-Bank-Instance' end}
        bankOpen=true;accountVisible=true;freeSlots=1
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    for source in [
        include_str!("../addon/Collectors.lua"),
        include_str!("../addon/Core.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load("AO.Capture()").exec()?;
    let read = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        let hex: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{hex}\"").as_bytes(),
        )?)
    };
    let snapshot = read()?;
    assert_eq!(
        snapshot.sections["character_bank"]["6"]["1"]["guid"],
        "Item-Bank-Instance"
    );
    assert_eq!(
        snapshot.sections["character_bank"]["6"]["1"]["link"],
        "item:744:7"
    );
    assert_eq!(snapshot.sections["account_bank"], serde_json::json!({}));
    let summary = azeroth_operator::analysis::summary(&snapshot);
    assert_eq!(summary["character_bank_item_slots_observed"], 1);
    assert_eq!(summary["account_bank_item_slots_observed"], 0);
    let mut overlap = snapshot.clone();
    overlap.sections.insert(
        "account_bank".into(),
        snapshot.sections["character_bank"].clone(),
    );
    assert!(overlap.validate().is_err());
    for setup in [
        "bankOpen=false",
        "bankOpen=true;freeSlots=2",
        "freeSlots=1;malformed=true",
    ] {
        lua.load(format!("{setup};AO.Capture()")).exec()?;
        let snapshot = read()?;
        assert!(!snapshot.sections.contains_key("character_bank"));
        assert!(snapshot.unavailable.contains_key("character_bank"));
        assert!(
            azeroth_operator::analysis::summary(&snapshot)["character_bank_item_slots_observed"]
                .is_null()
        );
    }
    Ok(())
}
