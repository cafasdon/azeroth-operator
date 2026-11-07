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
        Enum={BankType={Character=0,Account=2},TradeskillRecipeType={Item=1,Salvage=2},
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
            GetContainerNumSlots=function(bag) return bag==6 and 2 or 0 end,
            GetContainerNumFreeSlots=function() return freeSlots end,
            GetContainerItemInfo=function(bag,slot)
                if bag==6 and slot==1 then return {itemID=744,stackCount=malformed and 0 or 1,hyperlink='item:744:7'} end
            end
        }
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end}
        C_Item={GetItemGUID=function() return 'Item-Bank-Instance' end}
        Professions={InLocalCraftingMode=function() return localCrafting end}
        C_TradeSkillUI={IsNPCCrafting=function() return npcCrafting end}
        local recipeInfo={recipeID=100,name='Fixture recipe',learned=true}
        recipeSchematic={recipeID=100,outputItemID=456,isRecraft=false,recipeType=1,reagentSlotSchematics={
            {required=true,reagents={{itemID=744}},quantityRequired=2,variableQuantities={}},
            {required=true,reagents={{itemID=744}},quantityRequired=1,variableQuantities={}},
            {required=false,reagents={{itemID=999}},quantityRequired=5,variableQuantities={}}
        }}
        ProfessionsFrame={CraftingPage={SchematicForm={recipeSchematic=recipeSchematic,
            IsVisible=function() return recipeVisible end,GetRecipeInfo=function() return recipeInfo end}}}
        localCrafting=true;recipeVisible=true
        C_QuestLog={
            GetNumQuestLogEntries=function() return 3,2 end,
            GetInfo=function(index)
                if index==1 then return {isHeader=true} end
                return {isHeader=false,questID=40+index,title="Fixture quest",level=10}
            end,
            ReadyForTurnIn=function(id) if id==42 then return false end end,
            IsFailed=function() return false end,
            GetNumQuestObjectives=function() return 1 end,
            GetQuestObjectives=function(id)
                if id==43 then return end
                local objective={type="item",finished=false,numFulfilled=0,numRequired=5,text="Private text"}
                return sparseObjectives and {[2]=objective} or {objective}
            end,
        }
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
            IsFactionParagonForCurrentPlayer=function() return false end,
        }
        C_GossipInfo={GetFriendshipReputation=function() return {friendshipFactionID=0} end}
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
            return {currencyID=id,name="Currency "..id,isHeader=false,isTypeUnused=id==200,
                quantity=0,maxQuantity=100,maxWeeklyQuantity=40,quantityEarnedThisWeek=10,
                totalEarned=60,isAccountWide=id==100, useTotalEarnedForMaxQty=true}
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
    let quests = &snapshot.sections["quests"];
    assert_eq!(quests["42"]["ready_for_turn_in"], false);
    assert_eq!(
        quests["42"]["objectives"]["1"],
        serde_json::json!({
            "kind":"item","finished":false,"fulfilled":0,"required":5
        })
    );
    assert!(quests["43"].get("objectives").is_none());
    assert!(quests["43"].get("ready_for_turn_in").is_none());
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
    lua.load("currencyReordered=true;AO.Capture()").exec()?;
    assert_eq!(&read()?.sections["currency_balances"], currencies);
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
        azeroth_operator::analysis::summary(&snapshot)["reputation_factions_observed"],
        2
    );
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
    let recipe = azeroth_operator::analysis::review_recipe(&snapshot, 2)?;
    assert_eq!(recipe["materials"][0]["required"], 6);
    assert_eq!(recipe["materials"][0]["quantity_observed"], 1);
    assert_eq!(recipe["materials"][0]["shortfall_vs_observed"], 5);
    assert_eq!(recipe["craftability_verified"], false);
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
