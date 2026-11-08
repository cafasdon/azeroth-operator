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
        AO.family='classic'; assert(not pcall(AO.collectors.action_bar_observation)); AO.family='retail'
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
    verify_forever_capture_fixture(None, 16001)
}

pub fn verify_forever_capture_fixture(
    staged: Option<&std::path::Path>,
    interface: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("expected_interface", interface)?;
    lua.load(r#"
        AO={family='forever',variant='live',interface=expected_interface};SlashCmdList={}
        function print(message) export_notice=message end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',expected_interface end
        function UnitName() return 'Example' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        walletCalls=0;walletCopper=0
        function GetMoney() walletCalls=walletCalls+1; return walletCopper end
        function UnitLevel() return 60 end
        function UnitClass() return 'Hunter','HUNTER' end
        function UnitRace() return 'Orc','Orc' end
        factionTag='Horde'
        function UnitFactionGroup(unit) assert(unit=='player'); return factionTag end
        function HasNewMail() return fixtureNewMail end
        function GetBindLocation() return fixtureBindName or "Fixture Inn" end
        function GetInstanceInfo() return "Fixture Dungeon",fixtureInstanceKind or "party",1,"Normal",5,nil,nil,123 end
        fixtureLockDialog={IsVisible=function() return fixtureLockVisible end}
        function StaticPopup_FindVisible(key) assert(key=="INSTANCE_LOCK"); return fixtureLockVisible and fixtureLockDialog end
        function GetInstanceLockTimeRemaining() assert(fixtureLockVisible); return 14.1,false,8,3 end
        function GetInstanceLockTimeRemainingEncounter(index)
            assert(fixtureLockVisible and index>=1 and index<=8)
            return "Boss "..index,"Fixture texture",index<=3 and not fixtureBadEncounter,true
        end
        fixtureNewMail=true
        C_Map={GetBestMapForUnit=function(unit) assert(unit=="player"); return 1411 end,
            GetMapInfo=function(id) assert(id==1411); return {mapID=1411,name="Fixture zone",parentMapID=1414} end,
            GetMapLevels=function(id) assert(id==1411); return 5,12,1,2 end}
        function GetInventoryItemID() end
        NUM_TOTAL_EQUIPPED_BAG_SLOTS=4
        Enum={BankType={Character=0,Account=2},TradeskillRecipeType={Item=1,Salvage=2,Enchant=3},
            ContentTrackingType={Achievement=2}}
        BankFrame={IsShown=function() return bankOpen end}
        C_Bank={
            CanViewBank=function(kind) return kind==0 or accountVisible end,
            FetchNumPurchasedBankTabs=function(kind) return kind==0 and 1 or 0 end,
            FetchPurchasedBankTabData=function(kind)
                return kind==0 and {{ID=6,bankType=0}} or {}
            end
        }
        C_Container={
            PlayerHasHearthstone=function() if not fixtureNoHearth then return 6948 end end,
            GetItemCooldown=function(id)
                assert(id==6948)
                if fixtureReadyHearth then return 0,0,1 end
                return 10,30.5,fixtureDisabledHearth and 0 or 1
            end,
            GetContainerNumSlots=function(bag)
                if bag==0 then return 16 end
                if bag==1 then return 8 end
                return bag==6 and 2 or 0
            end,
            GetContainerNumFreeSlots=function(bag)
                if bag==0 then return badCapacity and 17 or 16,0 end
                if bag==1 then return 8,32 end
                assert(bag==6)
                return freeSlots
            end,
            GetContainerItemInfo=function(bag,slot)
                if bag==6 and slot==1 then return {itemID=744,stackCount=malformed and 0 or 1,hyperlink='item:744:7'} end
            end
        }
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end}
        GetTime=function() return 20 end
        C_Item={GetItemGUID=function() return 'Item-Bank-Instance' end}
        Professions={InLocalCraftingMode=function() return localCrafting end}
        C_TradeSkillUI={IsNPCCrafting=function() return npcCrafting end,
            GetRecipesTracked=function(recraft)
                if malformedTrackedRecipes then return {[2]=100} end
                if recraft then return {100} end
                return {100,101,102}
            end,
            GetRecipeInfo=function(id)
                if id==100 then return {recipeID=id,name="Tracked fixture",learned=false} end
                if id==101 then return {recipeID=id,name="Restricted fixture",learned=true} end
                if id==102 then error("Unavailable metadata") end
            end,
            GetRecipeCooldown=function(id) assert(id==100); return 12.1,false,1,3 end,
            GetProfessionInfoByRecipeID=function(id)
                assert(id==100)
                return {professionID=164,professionName="Blacksmithing",skillLevel=12,maxSkillLevel=75}
            end,
            GetRecipeRequirements=function(id)
                assert(id==100)
                if malformedRequirements then return {[2]={name="Fixture area",met=false,type=2}} end
                if emptyRequirements then return {} end
                return {{name="Fixture station",met=false,type=0},{name="Fixture tool",met=true,type=1}}
            end}
        local recipeInfo={recipeID=100,name='Fixture recipe',learned=true,
            relativeDifficulty=0,canSkillUp=true,numSkillUps=2}
        recipeSchematic={recipeID=100,outputItemID=456,quantityMin=2,quantityMax=3,isRecraft=false,recipeType=1,reagentSlotSchematics={
            {required=true,reagents={{itemID=744}},quantityRequired=2,variableQuantities={}},
            {required=true,reagents={{itemID=744}},quantityRequired=1,variableQuantities={}},
            {required=false,reagents={{itemID=999}},quantityRequired=5,variableQuantities={}}
        }}
        ProfessionsFrame={CraftingPage={SchematicForm={recipeSchematic=recipeSchematic,
            IsVisible=function() return recipeVisible end,GetRecipeInfo=function() return recipeInfo end}}}
        localCrafting=true;recipeVisible=true
        GameLimitedMode_IsBankedXPActive=function() return trialXP==true end
        GameLimitedMode_GetLevelLimit=function() return 20 end
        GameRulesUtil={IsPlayerAtEffectiveMaxLevel=function() return cappedXP==true end}
        UnitXP=function(unit) assert(unit=="player"); return 0 end
        UnitXPMax=function(unit) assert(unit=="player"); return 1000 end
        IsXPUserDisabled=function() return false end
        GetXPExhaustion=function() return 200 end
        IsResting=function() return fixtureResting==true end
        C_QuestLog={
            GetNumQuestLogEntries=function() return 3,2 end,
            GetInfo=function(index)
                if index==1 then return {isHeader=true} end
                return {isHeader=false,questID=40+index,title="Fixture quest",level=10}
            end,
            ReadyForTurnIn=function(id) if id==42 then return false end end,
            IsFailed=function() return false end,
            IsEliteQuest=function(id) return id==42 end,
            GetSuggestedGroupSize=function(id) return id==42 and 3 or 0 end,
            GetSelectedQuest=function() return 42 end,
            ShouldShowQuestRewards=function(id) return id==42 end,
            GetQuestLogMajorFactionReputationRewards=function(id)
                assert(id==42)
                return {{factionID=20,rewardAmount=invalidItemChoice and -1 or 75}}
            end,
            GetQuestRewardCurrencyInfo=function(id,index,isChoice)
                assert(id==42 and index==2 and isChoice==true)
                return {currencyID=18,totalRewardAmount=50}
            end,
            GetRequiredMoney=function(id)
                if invalidQuestMoney then return -1 end
                return id==42 and 250 or 0
            end,
            GetTimeAllowed=function(id)
                if id==43 then return end
                if invalidItemChoice then return 100,105 end
                return 100,25.5
            end,
            GetNextWaypoint=function(id)
                if id==43 then error("unavailable") end
                if invalidQuestMap then return 0 end
                return 1411,0.25,0.75
            end,
            GetNumQuestObjectives=function() return 1 end,
            GetQuestObjectives=function(id)
                if id==43 then return end
                local objective={type="item",finished=false,numFulfilled=0,numRequired=5,text="Private text"}
                return sparseObjectives and {[2]=objective} or {objective}
            end,
        }
        GetNumQuestLogChoices=function(id,all) assert(id==42 and all==true); return 2 end
        GetNumQuestLogRewards=function() return 1 end
        GetQuestLogRewardMoney=function() return 123 end
        GetQuestLogRewardXP=function() return invalidItemChoice and -1 or 456 end
        GetQuestLogRewardHonor=function() return 0 end
        GetQuestLogRewardInfo=function(index) assert(index==1); return "Private",0,3,1,true,117 end
        GetQuestLogChoiceInfo=function(index)
            assert(index==1)
            if invalidItemChoice then return end
            return "Private",0,1,1,true,118
        end
        GetQuestLogChoiceInfoLootType=function(index) return invalidQuestReward and 2 or index-1 end
        MAX_ACTION_BUTTONS=nil
        NUM_ACTIONBAR_BUTTONS=2
        ActionButtonUtil={ActionBarButtonNames={"ActionButton","MultiBarBottomLeftButton"}}
        ActionButton1={action=1,bindingAction="ACTIONBUTTON1"}
        ActionButton2={action=2,bindingAction="ACTIONBUTTON2"}
        MultiBarBottomLeftButton1={action=1,bindingAction="MULTIACTIONBAR1BUTTON1"}
        GetBindingKey=function(command)
            if command=="ACTIONBUTTON1" then return "1","SHIFT-1" end
            if command=="CLICK MultiBarBottomLeftButton1:LeftButton" then return "ALT-1" end
        end
        GetActionInfo=function(slot) if slot==1 then return "spell",101 end end
        Enum.SpellBookSpellBank={Player=0}
        Enum.FlightPathState={Current=0,Reachable=1,Unreachable=2}
        FlightMapFrame={IsVisible=function() return not closedFlightMap end,GetMapID=function() return 1414 end}
        C_TaxiMap={GetAllTaxiNodes=function(map)
            assert(map==1414 and not closedFlightMap)
            return {{slotIndex=1,nodeID=12,name="Fixture Origin",state=0,isMapLayerTransition=false},
                {slotIndex=3,nodeID=14,name="Fixture Stop",state=1,isMapLayerTransition=false,position={x=1,y=2}},
                {slotIndex=4,nodeID=15,name="Fixture Layer",state=1,isMapLayerTransition=true}}
        end}
        TaxiNodeCost=function(slot) assert(slot==3); return 250 end
        Enum.GameRule={RepairArmorDisabled=42}
        MerchantFrame={IsVisible=function() return not closedMerchant end,selectedTab=1}
        ClassTrainerFrame={IsVisible=function() return not closedTrainer end}
        Enum.TrainerType={General=0,Tradeskills=2,Pet=3}
        C_Trainer={GetTrainerType=function() return fixtureTrainerKind or 2 end}
        function GetNumTrainerServices() return 2 end
        function GetTrainerServiceInfo(index)
            if index==1 then return "Fixture lesson","available",nil,10,"Rank 2" end
            return "Known lesson","used"
        end
        function GetTrainerServiceCost(index) assert(index==1); return 500 end
        function GetTrainerServiceSkillReq(index) assert(index==1); return "Fixture skill",50,true end
        function GetTrainerServiceNumAbilityReq(index) assert(index==1); return fixtureEmptyAbilities and 0 or 1 end
        function GetTrainerServiceAbilityReq(index,ability) assert(index==1 and ability==1); return "Fixture prerequisite",false end
        GetMerchantNumItems=function() return 2 end
        GetMerchantItemID=function(index) assert(index==1); return 744 end
        C_MerchantFrame={GetItemInfo=function(index)
            if index==1 then return {price=250,stackCount=5,numAvailable=-1,hasExtendedCost=false} end
            return {price=0,stackCount=1,numAvailable=0,hasExtendedCost=true,currencyID=99}
        end}
        function GetMerchantItemCostInfo(index) assert(index==2); return 2 end
        function GetMerchantItemCostItem(index,component)
            assert(index==2)
            if component==1 then return 123,3,"item:900" end
            return 456,fixtureBadMerchantCost and 0 or 7,nil,"Fixture Marks"
        end
        C_Item.GetItemInfoInstant=function(link) assert(link=="item:900"); return 900 end
        CanMerchantRepair=function() return not nonRepairMerchant end
        GetRepairAllCost=function() assert(not closedMerchant and not nonRepairMerchant); return fixtureRepairCost or 325,false end
        C_GameRules={IsGameRuleActive=function(rule) assert(rule==42); return repairsDisabled==true end}
        GetInventoryAlertStatus=function(index)
            assert(index>=1 and index<=11)
            if clearDurability then return 0 end
            if index==3 then return 1 end
            if index==10 then return 2 end
            return 0
        end
        Enum.SpellBookItemType={Spell=1,FutureSpell=2}
        C_Spell={GetSpellName=function(spell) if spell==101 then return "Fixture spell" end end}
        C_SpellBook={IsSpellInSpellBook=function(spell,bank,overrides)
            assert(bank==0 and overrides==true)
            return spell==102
        end,GetNumSpellBookSkillLines=function() return 3 end,
        GetSpellBookSkillLineInfo=function(line)
            if line==1 then return {isGuild=false,shouldHide=false,itemIndexOffset=2,numSpellBookItems=2} end
            if line==2 then return {isGuild=true,shouldHide=false,itemIndexOffset=4,numSpellBookItems=1} end
            if malformedSpellbook then return {isGuild=false,shouldHide=false,itemIndexOffset=2,numSpellBookItems=1} end
            return {isGuild=false,shouldHide=true,itemIndexOffset=5,numSpellBookItems=1}
        end,GetSpellBookItemInfo=function(slot,bank)
            assert(bank==0)
            if slot==3 then return {itemType=1,actionID=100,spellID=101,name="Fixture spell",isPassive=false,isOffSpec=true} end
            if slot==4 then return {itemType=2,actionID=102,spellID=102} end
            error("Non-player line queried")
        end}
        C_QuestInfoSystem={GetQuestRewardSpells=function(id)
            if id==43 then return {} end
            return invalidItemChoice and {101,0} or {101,102}
        end,GetQuestRewardSpellInfo=function(id,spell)
            assert(id==42)
            if spell==102 then error("Not loaded") end
            return {type=0,isSpellLearned=false,name="Private"}
        end,GetQuestRewardCurrencies=function(id)
            if id==43 then return {} end
            local row={currencyID=17,totalRewardAmount=25,name="Do not export"}
            return invalidQuestReward and {row,{currencyID=18}} or {row}
        end}
        local function faction(id)
            return {factionID=id,name="Fixture "..id,isHeader=id==20,isHeaderWithRep=id==20,
                isAccountWide=id==20,reaction=4,currentStanding=-50,currentReactionThreshold=-100,nextReactionThreshold=0}
        end
        C_Reputation={
            GetNumFactions=function() return 3 end,
            GetFactionDataByIndex=function(index)
                if index==1 then return {isHeader=true,isHeaderWithRep=false} end
                if index==2 then return faction(10) end
                if not missingFaction then return faction(20) end
            end,
            IsMajorFaction=function(id) if classificationMissing then error("unavailable") end return id==20 end,
            IsFactionParagonForCurrentPlayer=function(id) return id==10 and paragonEnabled==true end,
            GetFactionParagonInfo=function(id)
                assert(id==10)
                return 250,100,314,true,false,2
            end,
        }
        C_GossipInfo={GetFriendshipReputation=function(id)
            if id==10 and friendshipEnabled then
                local nextThreshold
                if not friendshipCapped then nextThreshold=100 end
                return {friendshipFactionID=15,standing=35,reactionThreshold=20,
                    nextThreshold=nextThreshold,text="Private dialogue"}
            end
            return {friendshipFactionID=0}
        end}
        C_MajorFactions={
            GetMajorFactionData=function(id)
                assert(id==20)
                if not missingRenown then
                    return {renownLevel=5,renownReputationEarned=12,renownLevelThreshold=100}
                end
            end,
            HasMaximumRenown=function(id) assert(id==20);return renownCapped==true end,
        }
        ACHIEVEMENT_FLAGS_ACCOUNT=131072
        bit={band=function(flags,mask) return math.floor(flags/mask)%2*mask end}
        tracked={101,102}
        C_ContentTracking={GetTrackedIDs=function(kind) assert(kind==Enum.ContentTrackingType.Achievement); return tracked end}
        function GetAchievementInfo(id)
            return id,"Fixture achievement",5,true,1,1,2025,"Description",ACHIEVEMENT_FLAGS_ACCOUNT,0,"Reward",id==102,false,"Private character"
        end
        function GetAchievementNumCriteria(id) assert(id==101);return 2 end
        function GetAchievementCriteriaInfo(id,index)
            assert(id==101)
            return "Private objective",0,index==1,index==1 and 2 or nil,2,"Private contributor",0,0,"Private quantity",300+index
        end
        local function currency(id)
            local weekly=id==100
            if missingWeekly then weekly=nil end
            return {currencyID=id,name="Currency "..id,isHeader=false,isTypeUnused=id==200,
                quantity=0,maxQuantity=100,maxWeeklyQuantity=40,quantityEarnedThisWeek=10,
                totalEarned=60,isAccountWide=id==100, useTotalEarnedForMaxQty=true,canEarnPerWeek=weekly}
        end
        C_CurrencyInfo={
            GetCurrencyListSize=function() return 3 end,
            GetCurrencyListInfo=function(index)
                if index==1 then return {isHeader=true} end
                if index==2 then return currency(currencyReordered and 200 or 100) end
                if not missingCurrency then return currency((currencyReordered or duplicateCurrency) and 100 or 200) end
            end,
        }
        bankOpen=true;accountVisible=true;freeSlots=1
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    if let Some(root) = staged {
        lua.load(std::fs::read_to_string(root.join("Adapter.lua"))?)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    for (name, source) in [
        ("Collectors.lua", include_str!("../addon/Collectors.lua")),
        ("Core.lua", include_str!("../addon/Core.lua")),
    ] {
        let source = match staged {
            Some(root) => std::fs::read_to_string(root.join(name))?,
            None => source.to_owned(),
        };
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(
        r#"
        assert(SlashCmdList.AZEROTHOPERATOR and not SlashCmdList.AZEROTHOPERATORMACROS)
        SlashCmdList.AZEROTHOPERATOR()
        assert(export_notice:find("snapshot captured",1,true))
    "#,
    )
    .exec()?;
    let read = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        let hex: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{hex}\"").as_bytes(),
        )?)
    };
    let snapshot = read()?;
    assert_eq!(snapshot.sections["stats"]["faction"], "Horde");
    lua.load("factionTag='Neutral';SlashCmdList.AZEROTHOPERATOR()")
        .exec()?;
    assert!(read()?.sections["stats"].get("faction").is_none());
    lua.load("UnitFactionGroup=function() error('unavailable') end;SlashCmdList.AZEROTHOPERATOR()")
        .exec()?;
    assert_eq!(read()?.sections["stats"]["level"], 60);
    assert!(!snapshot.sections.contains_key("wallet"));
    assert_eq!(lua.globals().get::<u32>("walletCalls")?, 0);
    lua.load("SlashCmdList.AZEROTHOPERATOR('wallet')").exec()?;
    assert_eq!(read()?.sections["wallet"]["copper"], 0);
    lua.load("walletCopper=9999999999999;SlashCmdList.AZEROTHOPERATOR('wallet')")
        .exec()?;
    assert_eq!(read()?.sections["wallet"]["copper"], 9_999_999_999_999u64);
    lua.load("walletCopper=-1;SlashCmdList.AZEROTHOPERATOR('wallet')")
        .exec()?;
    assert!(read()?.unavailable.contains_key("wallet"));
    lua.load("SlashCmdList.AZEROTHOPERATOR()").exec()?;
    assert!(!read()?.unavailable.contains_key("wallet"));
    let quests = &snapshot.sections["quests"];
    assert_eq!(
        snapshot.sections["flight_map"],
        serde_json::json!({"map_id":1414,"entries":{
            "1":{"node_id":12,"name":"Fixture Origin","state":"current","map_transition":false},
            "3":{"node_id":14,"name":"Fixture Stop","state":"reachable","map_transition":false,"fare_copper":250},
            "4":{"node_id":15,"name":"Fixture Layer","state":"reachable","map_transition":true}
        }})
    );
    lua.load("closedFlightMap=true;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("flight_map"));
    lua.load("closedFlightMap=false").exec()?;
    assert_eq!(
        snapshot.sections["durability_alerts"],
        serde_json::json!({"chest":"low","off_hand":"broken"})
    );
    lua.load("clearDurability=true;AO.Capture()").exec()?;
    assert_eq!(read()?.sections["durability_alerts"], serde_json::json!({}));
    lua.load("repairsDisabled=true;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("durability_alerts"));
    lua.load("repairsDisabled=false;clearDurability=false")
        .exec()?;
    assert_eq!(
        snapshot.sections["repair_quote"],
        serde_json::json!({"copper":325})
    );
    lua.load("fixtureRepairCost=0;AO.Capture()").exec()?;
    assert_eq!(read()?.sections["repair_quote"]["copper"], 0);
    assert_eq!(
        snapshot.sections["trainer_services"],
        serde_json::json!({"kind":"tradeskills","entries":{
            "1":{"name":"Fixture lesson","status":"available","required_level":10,"rank_label":"Rank 2","price_copper":500,
                "skill_requirement":{"name":"Fixture skill","required_rank":50,"met":true},
                "ability_requirements":{"1":{"name":"Fixture prerequisite","met":false}}},
            "2":{"name":"Known lesson","status":"used"}
        }})
    );
    lua.load("fixtureEmptyAbilities=true;AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["trainer_services"]["entries"]["1"]["ability_requirements"],
        serde_json::json!({})
    );
    lua.load("fixtureEmptyAbilities=false").exec()?;
    lua.load("fixtureTrainerKind=3;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("trainer_services"));
    lua.load("fixtureTrainerKind=nil;closedTrainer=true;AO.Capture()")
        .exec()?;
    assert!(read()?.unavailable.contains_key("trainer_services"));
    lua.load("closedTrainer=false").exec()?;
    assert_eq!(
        snapshot.sections["merchant_stock"],
        serde_json::json!({
            "1":{"item_id":744,"price_copper":250,"bundle_size":5,"available":-1,"extended_cost":false},
            "2":{"currency_id":99,"price_copper":0,"bundle_size":1,"available":0,"extended_cost":true,
                "cost_components":{"1":{"quantity":3,"item_id":900},"2":{"quantity":7,"currency_name":"Fixture Marks"}}}
        })
    );
    lua.load("fixtureBadMerchantCost=true;AO.Capture()")
        .exec()?;
    assert_eq!(
        read()?.sections["merchant_stock"]["2"]["extended_cost"],
        true
    );
    assert!(
        read()?.sections["merchant_stock"]["2"]
            .get("cost_components")
            .is_none()
    );
    lua.load("fixtureBadMerchantCost=false").exec()?;
    lua.load("MerchantFrame.selectedTab=2;AO.Capture()")
        .exec()?;
    assert!(read()?.unavailable.contains_key("repair_quote"));
    assert!(read()?.unavailable.contains_key("merchant_stock"));
    lua.load("MerchantFrame.selectedTab=1;closedMerchant=true;AO.Capture()")
        .exec()?;
    assert!(read()?.unavailable.contains_key("repair_quote"));
    lua.load("closedMerchant=false;nonRepairMerchant=true;AO.Capture()")
        .exec()?;
    assert!(read()?.unavailable.contains_key("repair_quote"));
    assert_eq!(
        snapshot.sections["action_bar_observation"],
        serde_json::json!({
            "coverage":"loaded_builtin_buttons_only","observed_buttons":3,"missing_buttons":1,
        "slots":{"1":{"empty":false,"kind":"spell","id":101},"2":{"empty":true}},
        "button_hotkeys":{"1":{"button":"ActionButton1","slot":1,"key1":"1","key2":"SHIFT-1"},
            "2":{"button":"ActionButton2","slot":2},"3":{"button":"MultiBarBottomLeftButton1","slot":1,"key1":"ALT-1"}}
        })
    );
    assert!(snapshot.unavailable.contains_key("action_bars"));
    assert_eq!(
        snapshot.sections["spellbook"],
        serde_json::json!({"coverage":"visible_player_spell_slots","entries":{"3":{"base_spell_id":100,"spell_id":101,"name":"Fixture spell","passive":false,"off_spec":true}}})
    );
    lua.load("malformedSpellbook=true;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("spellbook"));
    lua.load("malformedSpellbook=false").exec()?;
    assert_eq!(
        snapshot.sections["experience"],
        serde_json::json!({"status":"progress","current_xp":0,"level_threshold_xp":1000,"xp_disabled":false,"rested_xp":200,"resting":false})
    );
    lua.load("fixtureResting=true;AO.Capture()").exec()?;
    assert_eq!(read()?.sections["experience"]["resting"], true);
    lua.load("IsResting=nil;AO.Capture()").exec()?;
    assert!(read()?.sections["experience"].get("resting").is_none());
    lua.load("cappedXP=true;AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["experience"],
        serde_json::json!({"status":"level_capped"})
    );
    lua.load("cappedXP=false;trialXP=true;AO.Capture()")
        .exec()?;
    assert_eq!(
        read()?.sections["experience"],
        serde_json::json!({"status":"trial_capped"})
    );
    lua.load("trialXP=false").exec()?;
    assert_eq!(quests["42"]["ready_for_turn_in"], false);
    assert_eq!(
        quests["42"]["objectives"]["1"],
        serde_json::json!({
            "kind":"item","finished":false,"fulfilled":0,"required":5
        })
    );
    assert!(quests["43"].get("objectives").is_none());
    assert!(quests["43"].get("ready_for_turn_in").is_none());
    assert_eq!(quests["42"]["next_waypoint_map_id"], 1411);
    assert_eq!(snapshot.sections["stats"]["map_id"], 1411);
    assert_eq!(snapshot.sections["stats"]["map_name"], "Fixture zone");
    assert_eq!(snapshot.sections["stats"]["map_parent_id"], 1414);
    let summary = azeroth_operator::analysis::summary(&snapshot);
    assert_eq!(
        summary["instance_context"],
        serde_json::json!({"kind":"party","name":"Fixture Dungeon","difficulty_id":1,"instance_map_id":123})
    );
    lua.load("fixtureInstanceKind='none';AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["instance_context"],
        serde_json::json!({"kind":"none"})
    );
    lua.load("fixtureInstanceKind='unknown';AO.Capture()")
        .exec()?;
    assert!(read()?.unavailable.contains_key("instance_context"));
    lua.load("fixtureInstanceKind=nil").exec()?;
    lua.load("fixtureLockVisible=true;AO.Capture()").exec()?;
    let warning = read()?.sections["instance_context"]["lock_warning"].clone();
    assert_eq!(
        warning["encounters"]["3"],
        serde_json::json!({"name":"Boss 3","killed":true})
    );
    assert_eq!(
        warning["encounters"]["8"],
        serde_json::json!({"name":"Boss 8","killed":false})
    );
    lua.load("fixtureBadEncounter=true;AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["instance_context"]["lock_warning"],
        serde_json::json!({
            "remaining_seconds":15,"encounters_total":8,"encounters_completed":3
        })
    );
    lua.load("fixtureLockVisible=false").exec()?;
    assert_eq!(summary["new_mail_indicator"], true);
    assert_eq!(summary["hearthstone_bind_name"], "Fixture Inn");
    assert_eq!(summary["hearthstone_item_id"], 6948);
    assert_eq!(summary["hearthstone_cooldown_seconds"], 21);
    lua.load("fixtureReadyHearth=true;AO.Capture()").exec()?;
    assert_eq!(read()?.sections["stats"]["hearthstone_cooldown_seconds"], 0);
    lua.load("fixtureReadyHearth=false;fixtureDisabledHearth=true;AO.Capture()")
        .exec()?;
    assert!(
        read()?.sections["stats"]
            .get("hearthstone_cooldown_seconds")
            .is_none()
    );
    lua.load("fixtureNoHearth=true;AO.Capture()").exec()?;
    assert!(
        read()?.sections["stats"]
            .get("hearthstone_item_id")
            .is_none()
    );
    lua.load("fixtureBindName='';AO.Capture()").exec()?;
    assert!(
        read()?.sections["stats"]
            .get("hearthstone_bind_name")
            .is_none()
    );
    lua.load("fixtureBindName=nil").exec()?;
    lua.load("fixtureNewMail=nil;AO.Capture()").exec()?;
    assert_eq!(read()?.sections["stats"]["new_mail_indicator"], false);
    lua.load("HasNewMail=nil;AO.Capture()").exec()?;
    assert!(
        read()?.sections["stats"]
            .get("new_mail_indicator")
            .is_none()
    );
    assert_eq!(
        snapshot.sections["bag_capacity"]["1"],
        serde_json::json!({"total_slots":8,"free_slots":8,"family_mask":32})
    );
    assert_eq!(
        summary["carried_bag_space"],
        serde_json::json!({"total_slots":24,"free_slots":24,"known_general_free_slots":16})
    );
    lua.load("badCapacity=true;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("bag_capacity"));
    lua.load("badCapacity=false").exec()?;
    assert_eq!(summary["captured_map_min_level"], 5);
    assert_eq!(summary["captured_map_max_level"], 12);
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["quests_with_waypoint_on_captured_map"],
        serde_json::json!([42])
    );
    assert_eq!(quests["42"]["required_copper"], 250);
    assert_eq!(quests["43"]["required_copper"], 0);
    assert_eq!(quests["42"]["suggested_group_size"], 3);
    assert_eq!(quests["43"]["suggested_group_size"], 0);
    assert_eq!(quests["43"]["elite"], false);
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["observed_quest_difficulty"],
        serde_json::json!({"suggested_groups":[{"quest_id":42,"suggested_group_size":3}],"elite_quest_ids":[42]})
    );
    assert_eq!(
        quests["42"]["timer"],
        serde_json::json!({"total_seconds":100,"remaining_seconds":74})
    );
    assert!(quests["43"].get("timer").is_none());
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["observed_quest_timers"],
        serde_json::json!([{"quest_id":42,"remaining_seconds":74}])
    );
    assert_eq!(
        quests["42"]["currency_rewards"],
        serde_json::json!({"1":{"currency_id":17,"quantity":25}})
    );
    assert_eq!(quests["43"]["currency_rewards"], serde_json::json!({}));
    assert_eq!(
        quests["42"]["currency_choices"],
        serde_json::json!({"2":{"currency_id":18,"quantity":50}})
    );
    assert!(quests["43"].get("currency_choices").is_none());
    assert_eq!(
        quests["42"]["item_rewards"],
        serde_json::json!({"1":{"item_id":117,"quantity":3}})
    );
    assert_eq!(
        quests["42"]["item_choices"],
        serde_json::json!({"1":{"item_id":118,"quantity":1}})
    );
    assert!(quests["43"].get("item_rewards").is_none());
    assert_eq!(
        quests["42"]["reward_amounts"],
        serde_json::json!({"copper":123,"xp":456,"honor":0})
    );
    assert!(quests["43"].get("reward_amounts").is_none());
    assert_eq!(
        quests["42"]["renown_rewards"],
        serde_json::json!({"1":{"faction_id":20,"quantity":75}})
    );
    assert!(quests["43"].get("renown_rewards").is_none());
    assert_eq!(
        quests["42"]["spell_rewards"],
        serde_json::json!({"1":{"spell_id":101,"reward_type":0,"is_spell_learned":false,"name":"Fixture spell","in_player_spellbook":false},"2":{"spell_id":102,"in_player_spellbook":true}})
    );
    assert_eq!(quests["43"]["spell_rewards"], serde_json::json!({}));
    lua.load("invalidItemChoice=true;AO.Capture()").exec()?;
    let partial = read()?;
    assert_eq!(
        partial.sections["quests"]["42"]["timer"]["remaining_seconds"],
        0
    );
    assert!(
        partial.sections["quests"]["42"]
            .get("spell_rewards")
            .is_none()
    );
    assert!(
        partial.sections["quests"]["42"]
            .get("renown_rewards")
            .is_none()
    );
    assert_eq!(
        partial.sections["quests"]["42"]["reward_amounts"],
        serde_json::json!({"copper":123,"honor":0})
    );
    assert!(
        partial.sections["quests"]["42"]
            .get("item_choices")
            .is_none()
    );
    assert_eq!(
        partial.sections["quests"]["42"]["currency_choices"],
        quests["42"]["currency_choices"]
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["quest_money"],
        serde_json::json!({"quests_with_cost":{"42":{"required_copper":250,"shortfall_copper":null}},"cost_unknown_count":0})
    );
    assert!(quests["43"].get("next_waypoint_map_id").is_none());
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["quest_waypoint_maps"],
        serde_json::json!({"by_map_id":{"1411":[42]},"waypoint_map_unknown":[43]})
    );
    lua.load("invalidQuestMap=true;invalidQuestMoney=true;invalidQuestReward=true;AO.Capture()")
        .exec()?;
    assert!(
        read()?.sections["quests"]["42"]
            .get("currency_choices")
            .is_none()
    );
    assert!(
        read()?.sections["quests"]["42"]
            .get("currency_rewards")
            .is_none()
    );
    assert!(
        read()?.sections["quests"]["42"]
            .get("required_copper")
            .is_none()
    );
    assert!(
        read()?.sections["quests"]["42"]
            .get("next_waypoint_map_id")
            .is_none()
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["quest_turn_in"],
        serde_json::json!({"ready_observed":0,"status_observed":1,"status_unknown":1})
    );
    lua.load("sparseObjectives=true;AO.Capture()").exec()?;
    assert!(read()?.sections["quests"]["42"].get("objectives").is_none());
    let achievements = &snapshot.sections["tracked_achievements"]["entries"];
    assert_eq!(achievements["101"]["account_wide"], true);
    assert_eq!(achievements["101"]["completed"], true);
    assert_eq!(achievements["101"]["earned_by_character"], false);
    assert_eq!(
        achievements["101"]["criteria"]["1"],
        serde_json::json!({
            "criterion_id":301,"completed":true,"quantity":2,"required":2
        })
    );
    assert_eq!(
        achievements["101"]["criteria"]["2"],
        serde_json::json!({
            "criterion_id":302,"completed":false
        })
    );
    assert!(achievements.get("102").is_none());
    assert!(!achievements.to_string().contains("Private"));
    lua.load("tracked={[2]=101};AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("tracked_achievements"));
    lua.load("tracked={};AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["tracked_achievements"]["entries"],
        serde_json::json!({})
    );
    let currencies = &snapshot.sections["currency_balances"];
    assert_eq!(currencies["entries"]["100"]["quantity"], 0);
    assert_eq!(currencies["entries"]["100"]["account_wide"], true);
    assert_eq!(currencies["entries"]["100"]["cap_uses_total_earned"], true);
    assert_eq!(currencies["entries"]["100"]["total_earned"], 60);
    assert_eq!(currencies["entries"]["200"]["inactive"], true);
    assert_eq!(currencies["entries"]["100"]["can_earn_per_week"], true);
    assert_eq!(currencies["entries"]["200"]["can_earn_per_week"], false);
    lua.load("currencyReordered=true;AO.Capture()").exec()?;
    assert_eq!(&read()?.sections["currency_balances"], currencies);
    lua.load("missingWeekly=true;AO.Capture()").exec()?;
    let legacy = read()?;
    assert!(
        legacy.sections["currency_balances"]["entries"]["100"]
            .get("can_earn_per_week")
            .is_none()
    );
    lua.load(
        "missingWeekly=false; issecretvalue=function(v) return v==true end;
        local rows=AO.collectors.currency_balances().entries;
        assert(rows['100'].can_earn_per_week==nil and rows['200'].can_earn_per_week==false);
        issecretvalue=nil",
    )
    .exec()?;
    for setup in [
        "currencyReordered=false;duplicateCurrency=true",
        "missingCurrency=true",
    ] {
        lua.load(format!("{setup};AO.Capture()")).exec()?;
        assert!(read()?.unavailable.contains_key("currency_balances"));
    }
    let factions = &snapshot.sections["reputation"]["entries"];
    assert_eq!(factions["10"]["standing"], -50);
    assert_eq!(factions["10"]["kind"], "standard");
    assert_eq!(factions["20"]["kind"], "renown");
    assert_eq!(factions["20"]["account_wide"], true);
    assert_eq!(
        factions["20"]["renown"],
        serde_json::json!({
            "level":5,"earned_in_level":12,"level_threshold":100,"capped":false
        })
    );
    lua.load("renownCapped=true;AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["reputation"]["entries"]["20"]["renown"]["capped"],
        true
    );
    lua.load(
        "issecretvalue=function(v) return v==12 end;
        local rows=AO.collectors.reputation().entries;
        assert(rows['20'].kind=='renown' and rows['20'].renown==nil);
        issecretvalue=nil",
    )
    .exec()?;
    lua.load("missingRenown=true;AO.Capture()").exec()?;
    let without_renown = read()?;
    assert_eq!(
        without_renown.sections["reputation"]["entries"]["20"]["kind"],
        "renown"
    );
    assert!(
        without_renown.sections["reputation"]["entries"]["20"]
            .get("renown")
            .is_none()
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&snapshot)["reputation_factions_observed"],
        2
    );
    lua.load("friendshipEnabled=true;AO.Capture()").exec()?;
    let friendship = read()?;
    let faction = &friendship.sections["reputation"]["entries"]["10"];
    assert_eq!(faction["kind"], "friendship");
    assert_eq!(
        faction["friendship"],
        serde_json::json!({"standing":35,"threshold":20,"next_threshold":100})
    );
    lua.load(
        "issecretvalue=function(v) return v==35 end;
        local row=AO.collectors.reputation().entries['10'];
        assert(row.kind=='friendship' and row.friendship==nil);
        issecretvalue=nil;friendshipCapped=true;AO.Capture()",
    )
    .exec()?;
    assert_eq!(
        read()?.sections["reputation"]["entries"]["10"]["friendship"],
        serde_json::json!({"standing":35,"threshold":20})
    );
    lua.load("paragonEnabled=true;AO.Capture()").exec()?;
    let paragon = read()?;
    let faction = &paragon.sections["reputation"]["entries"]["10"];
    assert_eq!(faction["kind"], "paragon");
    assert_eq!(
        faction["paragon"],
        serde_json::json!({"current_value":250,"reward_threshold":100,"reward_quest_id":314,
            "reward_pending":true,"level_too_low":false})
    );
    assert!(faction.get("friendship").is_none());
    lua.load(
        "issecretvalue=function(v) return v==250 end;
        local row=AO.collectors.reputation().entries['10'];
        assert(row.kind=='paragon' and row.paragon==nil);
        issecretvalue=nil",
    )
    .exec()?;
    lua.load("classificationMissing=true;AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["reputation"]["entries"]["10"]["kind"],
        "unknown"
    );
    lua.load("missingFaction=true;AO.Capture()").exec()?;
    assert!(read()?.unavailable.contains_key("reputation"));
    assert_eq!(
        snapshot.sections["character_bank"]["6"]["1"]["guid"],
        "Item-Bank-Instance"
    );
    assert_eq!(
        snapshot.sections["character_bank"]["6"]["1"]["link"],
        "item:744:7"
    );
    assert_eq!(snapshot.sections["account_bank"], serde_json::json!({}));
    assert_eq!(
        snapshot.sections["selected_recipe"]["reagents"],
        serde_json::json!({"744":3})
    );
    assert_eq!(
        snapshot.sections["tracked_recipes"],
        serde_json::json!({
            "coverage":"player_tracked_recipes_only",
            "crafts":{"100":{"name":"Tracked fixture","learned":false},
                "101":{"name":"Restricted fixture","learned":true},"102":{}},
            "recrafts":{"100":{"name":"Tracked fixture","learned":false}}
        })
    );
    lua.load(
        "issecretvalue=function(v) return v=='Restricted fixture' end;
        assert(AO.collectors.tracked_recipes().crafts['101'].name==nil);
        issecretvalue=function(v) return v==true end;
        assert(AO.collectors.tracked_recipes().crafts['101'].learned==nil);
        assert(AO.collectors.tracked_recipes().crafts['100'].learned==false);
        issecretvalue=nil;malformedTrackedRecipes=true;
        assert(not pcall(AO.collectors.tracked_recipes));
        malformedTrackedRecipes=false",
    )
    .exec()?;
    let recipe = azeroth_operator::analysis::review_recipe(&snapshot, 2)?;
    assert_eq!(recipe["materials"][0]["required"], 6);
    assert_eq!(
        recipe["base_output_if_completed"],
        serde_json::json!({
            "item_id":456,"minimum":4,"maximum":6
        })
    );
    assert_eq!(recipe["materials"][0]["quantity_observed"], 1);
    assert_eq!(recipe["materials"][0]["shortfall_vs_observed"], 5);
    assert_eq!(recipe["craftability_verified"], false);
    assert_eq!(recipe["recipe"]["relative_difficulty"], "optimal");
    assert_eq!(recipe["recipe"]["can_skill_up"], true);
    assert_eq!(recipe["recipe"]["skill_ups_reported"], 2);
    assert_eq!(
        recipe["recipe"]["cooldown"],
        serde_json::json!({
            "remaining_seconds":13,"daily":false,"charges":1,"max_charges":3
        })
    );
    assert_eq!(recipe["shortfall_vs_observed_recipe_charges"], 1);
    assert_eq!(
        recipe["estimated_cooldown_end_at"],
        serde_json::json!(snapshot.captured_at + chrono::TimeDelta::seconds(13))
    );
    assert_eq!(recipe["profession_points_to_observed_cap"], 63);
    assert_eq!(recipe["recipe"]["profession"]["profession_id"], 164);
    assert_eq!(
        recipe["unmet_crafting_conditions"],
        serde_json::json!({
            "1":{"name":"Fixture station","met":false,"kind":"spell_focus"}
        })
    );
    lua.load(
        "malformedRequirements=true;
        assert(AO.collectors.selected_recipe().crafting_conditions==nil);
        malformedRequirements=false;emptyRequirements=true;
        assert(next(AO.collectors.selected_recipe().crafting_conditions)==nil);
        emptyRequirements=false;
        issecretvalue=function(v) return v=='Fixture station' end;
        assert(AO.collectors.selected_recipe().crafting_conditions==nil);
        issecretvalue=nil",
    )
    .exec()?;
    lua.load(
        "issecretvalue=function(v) return v==12.1 end;
        local cooldown=AO.collectors.selected_recipe().cooldown;
        assert(cooldown.remaining_seconds==nil and cooldown.charges==1);
        issecretvalue=nil",
    )
    .exec()?;
    lua.load(
        "issecretvalue=function(v) return v==3 end;
        local recipe=AO.collectors.selected_recipe();
        assert(recipe.output_item_id==456 and recipe.base_output_quantity==nil);
        issecretvalue=nil",
    )
    .exec()?;
    lua.load(
        "issecretvalue=function(v) return v==2 end;
        local recipe=AO.collectors.selected_recipe();
        assert(recipe.skill_ups_reported==nil and recipe.relative_difficulty=='optimal');
        issecretvalue=nil",
    )
    .exec()?;
    lua.load("recipeSchematic.recipeType=3;recipeSchematic.outputItemID=nil;AO.Capture()")
        .exec()?;
    let enchant = azeroth_operator::analysis::review_recipe(&read()?, 2)?;
    assert!(enchant["recipe"].get("output_item_id").is_none());
    assert!(enchant["base_output_if_completed"].is_null());
    assert_eq!(enchant["materials"], recipe["materials"]);
    assert_eq!(enchant["craftability_verified"], false);
    lua.load("recipeSchematic.recipeType=1").exec()?;
    lua.load("recipeSchematic.reagentSlotSchematics[1].reagents[2]={itemID=745};AO.Capture()")
        .exec()?;
    let unsupported = read()?;
    assert_eq!(
        unsupported.sections["selected_recipe"]["requirements"],
        "unsupported_required_slots"
    );
    assert_eq!(
        azeroth_operator::analysis::review_recipe(&unsupported, 1)?["materials"],
        serde_json::json!([])
    );
    lua.load("local reagents=recipeSchematic.reagentSlotSchematics[1].reagents; reagents[2]=nil; reagents.alternative={itemID=745};AO.Capture()").exec()?;
    assert_eq!(
        read()?.sections["selected_recipe"]["requirements"],
        "unsupported_required_slots"
    );
    for setup in [
        "recipeSchematic.recipeType=2",
        "recipeSchematic.recipeType=nil",
        "recipeSchematic.recipeType=1;recipeSchematic.reagentSlotSchematics[1]=nil",
        "recipeVisible=false",
        "recipeVisible=true;localCrafting=false",
        "localCrafting=true;npcCrafting=true",
    ] {
        lua.load(format!("{setup};AO.Capture()")).exec()?;
        assert!(read()?.unavailable.contains_key("selected_recipe"));
    }
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
