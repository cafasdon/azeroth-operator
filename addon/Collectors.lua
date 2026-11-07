local _, AO = ...
AO.collectors = {}
local function itemGUID(bag,slot)
    if (AO.family~="retail" and AO.family~="forever") or not (C_Item and C_Item.GetItemGUID and ItemLocation) then return nil end
    local ok,guid=pcall(function()
        local location=bag~=nil and ItemLocation:CreateFromBagAndSlot(bag,slot) or ItemLocation:CreateFromEquipmentSlot(slot)
        return C_Item.GetItemGUID(location)
    end)
    if not ok or (issecretvalue and issecretvalue(guid)) then return nil end
    if type(guid)=="string" and #guid>0 and #guid<=128 then return guid end
end
AO.collectors.operation_receipts = function()
    assert(AO.ReadMacroReceipts, "Receipt reader unavailable")
    return AO.ReadMacroReceipts()
end

AO.collectors.equipment = function()
    local equipment = {}
    for slot = 1, 19 do
        local id = GetInventoryItemID("player", slot)
        if id then
            equipment[tostring(slot)] = { item_id = id, link = GetInventoryItemLink("player", slot), guid=itemGUID(nil,slot) }
        end
    end
    return equipment
end

-- Raw native set observations only. A missing ID/location is unknown, not empty.
-- Optional observed_item describes a current location, not the set's internal GUID.
local function setItemIdentity(data)
    if data.ignored==true or not data.raw_item_id or data.raw_item_id<=0
        or not data.raw_location or data.raw_location<0 then return nil end
    if not (EquipmentManager_GetLocationData and ItemLocation and C_Item
        and C_Item.GetItemID and C_Item.GetItemGUID and C_Item.IsLocked) then return nil end
    local function readable(v)
        assert(not issecretvalue or not issecretvalue(v), "Restricted set item identity")
        return v
    end
    local function integer(v,lo,hi)
        v=readable(v)
        assert(type(v)=="number" and v%1==0 and v>=lo and v<=hi, "Invalid set item location")
        return v
    end
    local decoded=readable(EquipmentManager_GetLocationData(data.raw_location))
    assert(type(decoded)=="table", "Missing decoded location")
    -- Bank, unknown and non-player locations remain raw observations only.
    local player,bank,bags=readable(decoded.isPlayer),readable(decoded.isBank),readable(decoded.isBags)
    if player~=true or bank~=false or type(bags)~="boolean" then return nil end
    local result,location
    if bags then
        local maximum=integer(NUM_TOTAL_EQUIPPED_BAG_SLOTS,0,64)
        local bag=integer(decoded.bag,0,maximum)
        assert(C_Container and C_Container.GetContainerNumSlots, "Bag bounds unavailable")
        local size=integer(C_Container.GetContainerNumSlots(bag),0,1024)
        local slot=integer(decoded.slot,1,size)
        result={kind="bag",bag=bag,slot=slot}
        location=ItemLocation:CreateFromBagAndSlot(bag,slot)
    else
        local slot=integer(decoded.slot,1,19)
        result={kind="equipment",slot=slot}
        location=ItemLocation:CreateFromEquipmentSlot(slot)
    end
    local id=readable(C_Item.GetItemID(location))
    local guid=readable(C_Item.GetItemGUID(location))
    local locked=readable(C_Item.IsLocked(location))
    assert(id==data.raw_item_id and type(guid)=="string" and #guid>0 and #guid<=128
        and not guid:find("[%z\1-\31\127]") and type(locked)=="boolean", "Set item identity unavailable")
    assert(readable(C_Item.GetItemID(location))==id and readable(C_Item.GetItemGUID(location))==guid,
        "Set item changed during capture")
    result.item_id=id; result.guid=guid; result.locked=locked
    return result
end

AO.collectors.equipment_sets = function()
    local api=C_EquipmentSet
    assert(AO.family=="retail" and api and api.GetEquipmentSetIDs and api.GetNumEquipmentSets
        and api.GetEquipmentSetInfo and api.GetIgnoredSlots and api.GetItemIDs
        and api.GetItemLocations and api.GetEquipmentSetAssignedSpec and api.CanUseEquipmentSets,
        "Equipment set API unavailable")
    local function readable(v)
        assert(not issecretvalue or not issecretvalue(v), "Restricted equipment set observation")
        return v
    end
    local function number(v,low,high)
        v=readable(v)
        assert(type(v)=="number" and v%1==0 and v>=low and v<=high, "Invalid equipment set number")
        return v
    end
    local function boolean(v)
        v=readable(v); assert(type(v)=="boolean", "Invalid equipment set flag"); return v
    end
    local function catalogue()
        local count=number(api.GetNumEquipmentSets(),0,128)
        local ids=readable(api.GetEquipmentSetIDs())
        assert(type(ids)=="table", "Missing equipment set catalogue")
        local result,n={},0
        for key,id in pairs(ids) do
            number(key,1,128); id=number(id,0,4294967295)
            assert(not result[tostring(id)], "Duplicate equipment set ID")
            result[tostring(id)]=id; n=n+1
        end
        assert(n==count, "Incomplete equipment set catalogue")
        return result,count
    end
    local ids,count=catalogue()
    local result={coverage="native_catalogue_raw_slots",can_use=boolean(api.CanUseEquipmentSets()),entries={}}
    for key,id in pairs(ids) do
        local name,icon,returnedID,equipped,total,worn,inventory,lost,ignored=api.GetEquipmentSetInfo(id)
        name=readable(name)
        assert(type(name)=="string" and #name>0 and #name<=1024, "Missing equipment set name")
        assert(number(returnedID,0,4294967295)==id, "Equipment set identity changed")
        local entry={name=name,icon=number(icon,0,4294967295),is_equipped=boolean(equipped),
            num_items=number(total,0,19),num_equipped=number(worn,0,19),
            num_in_inventory=number(inventory,0,19),num_lost=number(lost,0,19),
            num_ignored=number(ignored,0,19),slots={}}
        local spec=readable(api.GetEquipmentSetAssignedSpec(id))
        if spec~=nil then entry.assigned_spec=number(spec,1,64) end
        local ignoreMap,itemMap,locationMap=readable(api.GetIgnoredSlots(id)),readable(api.GetItemIDs(id)),readable(api.GetItemLocations(id))
        assert(type(ignoreMap)=="table" and type(itemMap)=="table" and type(locationMap)=="table", "Missing equipment set slot map")
        for _,map in ipairs({ignoreMap,itemMap,locationMap}) do
            for slot in pairs(map) do number(slot,1,19) end
        end
        for slot=1,19 do
            local data={}
            local flag,item,location=readable(ignoreMap[slot]),readable(itemMap[slot]),readable(locationMap[slot])
            if flag~=nil then data.ignored=boolean(flag) end
            if item~=nil then data.raw_item_id=number(item,-2147483648,4294967295) end
            if location~=nil then data.raw_location=number(location,-2147483648,4294967295) end
            local resolved,identity=pcall(setItemIdentity,data)
            if resolved then data.observed_item=identity end
            entry.slots[tostring(slot)]=data
        end
        result.entries[key]=entry
    end
    local after,afterCount=catalogue()
    assert(count==afterCount, "Equipment set catalogue changed")
    for key in pairs(ids) do assert(after[key]~=nil, "Equipment set catalogue changed") end
    -- Optional complete index-to-ID map for later association preparation.
    -- Failure omits this field; it never guesses IDs from menu positions.
    local specOK,specializations=pcall(function()
        assert(type(GetNumSpecializations)=="function" and C_SpecializationInfo
            and type(C_SpecializationInfo.GetSpecializationInfo)=="function", "Specialisations unavailable")
        local size=number(GetNumSpecializations(),1,64)
        local map,seen={},{}
        for i=1,size do
            local id=number(C_SpecializationInfo.GetSpecializationInfo(i),1,4294967295)
            assert(not seen[id], "Duplicate specialisation ID")
            seen[id]=true; map[tostring(i)]=id
        end
        assert(number(GetNumSpecializations(),1,64)==size, "Specialisations changed")
        for i=1,size do
            assert(number(C_SpecializationInfo.GetSpecializationInfo(i),1,4294967295)==map[tostring(i)], "Specialisation identity changed")
        end
        for _,entry in pairs(result.entries) do
            assert(entry.assigned_spec==nil or map[tostring(entry.assigned_spec)], "Assigned specialisation unavailable")
        end
        return map
    end)
    if specOK then result.specializations=specializations end
    return result
end

local function containerItems(bag, capacity)
    local slots,occupied = {},0
    for slot = 1, capacity do
        local info = C_Container.GetContainerItemInfo(bag, slot)
        if info then
            slots[tostring(slot)] = {item_id=info.itemID,count=info.stackCount,link=info.hyperlink,guid=itemGUID(bag,slot)}
            occupied=occupied+1
        end
    end
    return slots,occupied
end

AO.collectors.bags = function()
    local bags = {}
    if C_Container and C_Container.GetContainerNumSlots and C_Container.GetContainerItemInfo then
        for bag = 0, (NUM_TOTAL_EQUIPPED_BAG_SLOTS or NUM_BAG_SLOTS or 4) do
            bags[tostring(bag)] = containerItems(bag,C_Container.GetContainerNumSlots(bag))
        end
        return bags
    end
    error("Container API unavailable")
end

local function bankItems(kind)
    assert(BankFrame and BankFrame:IsShown(), "Bank is not open")
    local bankType=Enum.BankType[kind]
    assert(bankType~=nil and C_Bank.CanViewBank(bankType), "Bank type unavailable")
    local count=C_Bank.FetchNumPurchasedBankTabs(bankType)
    local tabs=C_Bank.FetchPurchasedBankTabData(bankType)
    assert(type(count)=="number" and count>=0 and count<=64 and #tabs==count, "Bank tabs not loaded")
    assert(kind~="Character" or count>0, "Character bank tabs not loaded")
    local containers={}
    for _,tab in ipairs(tabs) do
        local bag=tab.ID
        assert(tab.bankType==bankType and type(bag)=="number" and bag%1==0
            and bag>(NUM_TOTAL_EQUIPPED_BAG_SLOTS or NUM_BAG_SLOTS or 4) and bag<=64
            and not containers[tostring(bag)], "Invalid bank tab")
        local capacity=C_Container.GetContainerNumSlots(bag)
        assert(type(capacity)=="number" and capacity%1==0 and capacity>=1 and capacity<=1024, "Bank slots not loaded")
        local slots,occupied=containerItems(bag,capacity)
        assert(C_Container.GetContainerNumFreeSlots(bag)==capacity-occupied, "Bank slots do not reconcile")
        for _,item in pairs(slots) do
            assert(type(item.item_id)=="number" and item.item_id%1==0 and item.item_id>=1 and item.item_id<=4294967295
                and type(item.count)=="number" and item.count%1==0 and item.count>=1 and item.count<=4294967295
                and (item.link==nil or (type(item.link)=="string" and #item.link<=4096)), "Bank item data unavailable")
        end
        containers[tostring(bag)]=slots
    end
    return containers
end

AO.collectors.character_bank = function() return bankItems("Character") end
AO.collectors.account_bank = function() return bankItems("Account") end

AO.collectors.stats = function()
    local version = GetBuildInfo()
    return { level = UnitLevel("player"), class = select(2, UnitClass("player")), race = select(2, UnitRace("player")), client_version = version }
end


AO.collectors.macros = function()
    assert(GetNumMacros and GetMacroInfo, "API missing")
    local account, character = GetNumMacros()
    local result = {}
    local function read(index, scope)
        local name, icon, body = GetMacroInfo(index)
        if name then result[tostring(index)] = {name=name, icon=icon, body=body, scope=scope} end
    end
    for index=1,account do read(index,"account") end
    local macroConsts = Constants and Constants.MacroConsts
    local accountLimit = (macroConsts and macroConsts.MAX_ACCOUNT_MACROS) or MAX_ACCOUNT_MACROS
    assert(type(accountLimit)=="number" and accountLimit>0 and accountLimit<=1024, "Macro boundary unavailable")
    for index=1,character do read(accountLimit+index,"character") end
    return result
end

AO.collectors.bindings = function()
    assert(GetNumBindings and GetBinding, "API missing")
    local result = {}
    for index=1,GetNumBindings() do
        local command, category, key1, key2 = GetBinding(index)
        if key1 or key2 then
            local context
            if AO.family=="retail" and C_KeyBindings and C_KeyBindings.GetBindingContextForAction then
                context=C_KeyBindings.GetBindingContextForAction(command) or 0
            end
            result[tostring(index)] = {command=command, category=category, key1=key1, key2=key2, context=context}
        end
    end
    return result
end

AO.collectors.binding_state = function()
    assert(AO.family=="retail" and GetCurrentBindingSet, "Binding state unavailable")
    local set=GetCurrentBindingSet()
    assert(set==1 or set==2, "Unsupported binding set")
    assert(C_KeyBindings and C_KeyBindings.GetBindingContextForAction, "Binding contexts unavailable")
    return {binding_set=set,coverage="bound_keys_only"}
end

AO.collectors.action_bars = function()
    assert(GetActionInfo and MAX_ACTION_BUTTONS, "Action slot boundary unavailable")
    local result = {}
    for slot=1,MAX_ACTION_BUTTONS do
        local kind, id, subType = GetActionInfo(slot)
        if kind then result[tostring(slot)] = {kind=kind, id=id, sub_type=subType} end
    end
    return result
end

-- Partial observations of Blizzard's loaded player action-button frames.
-- Other pages, addon replacement bars and special pet/stance bars are not covered.
AO.collectors.action_bar_observation = function()
    assert(AO.family=="retail" and GetActionInfo and ActionButtonUtil
        and type(ActionButtonUtil.ActionBarButtonNames)=="table", "Action button catalogue unavailable")
    assert(type(NUM_ACTIONBAR_BUTTONS)=="number" and NUM_ACTIONBAR_BUTTONS%1==0
        and NUM_ACTIONBAR_BUTTONS>=1 and NUM_ACTIONBAR_BUTTONS<=64, "Invalid button bound")
    local names=ActionButtonUtil.ActionBarButtonNames
    assert(#names>=1 and #names<=32, "Invalid action bar catalogue")
    local result={coverage="loaded_builtin_buttons_only",slots={},observed_buttons=0,missing_buttons=0}
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted action observation")
        return value
    end
    for _,prefix in ipairs(names) do
        assert(type(prefix)=="string" and #prefix<=128, "Invalid button prefix")
        for i=1,NUM_ACTIONBAR_BUTTONS do
            local button=_G[prefix..i]
            local slot=button and readable(button.action)
            if slot==nil then result.missing_buttons=result.missing_buttons+1
            else
                assert(type(slot)=="number" and slot%1==0 and slot>=1 and slot<=4096, "Invalid action slot")
                result.observed_buttons=result.observed_buttons+1
                local key=tostring(slot)
                if not result.slots[key] then
                    local kind,id,subType=GetActionInfo(slot)
                    kind,id,subType=readable(kind),readable(id),readable(subType)
                    if kind then result.slots[key]={empty=false,kind=kind,id=id,sub_type=subType}
                    else result.slots[key]={empty=true} end
                end
            end
        end
    end
    assert(result.observed_buttons>0, "No loaded action buttons")
    return result
end

local function loadoutExport(configID)
    if AO.family ~= "retail" then return {status="unavailable", reason="client_not_supported"} end
    local spec = C_SpecializationInfo
    if not (spec and spec.GetSpecialization and spec.GetSpecializationInfo and
        C_Traits.GenerateImportString and C_Traits.GetLoadoutSerializationVersion and
        C_Traits.ConfigHasStagedChanges) then
        return {status="unavailable", reason="api_unavailable"}
    end
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted value")
        return value
    end
    local staged = readable(C_Traits.ConfigHasStagedChanges(configID))
    assert(type(staged)=="boolean", "Unknown staged state")
    if staged then return {status="unavailable", reason="staged_changes"} end
    local index = readable(spec.GetSpecialization())
    local specID = readable(spec.GetSpecializationInfo(index))
    local version = readable(C_Traits.GetLoadoutSerializationVersion())
    local text = readable(C_Traits.GenerateImportString(configID))
    assert(type(specID)=="number" and specID>0 and type(version)=="number" and version>=0, "Invalid export identity")
    assert(type(text)=="string" and #text>0 and #text<=16384 and not text:find("[^A-Za-z0-9+/=]"), "Invalid export string")
    return {status="available", format="blizzard_traits", import_string=text,
        spec_id=specID, serialization_version=version, staged_changes=false}
end

local function savedLoadouts()
    assert(AO.family=="retail", "Unsupported client")
    local specID=C_SpecializationInfo.GetSpecializationInfo(C_SpecializationInfo.GetSpecialization())
    assert(type(specID)=="number" and specID>0,"No spec")
    local ids=C_ClassTalents.GetConfigIDsBySpecID(specID)
    assert(type(ids)=="table" and #ids<=128,"Invalid loadout list")
    local selected=C_ClassTalents.GetLastSelectedSavedConfigID(specID)
    assert(selected==nil or (type(selected)=="number" and selected>0 and selected%1==0),"Invalid selected loadout")
    local result={spec_id=specID,selected_config_id=selected,entries={}}
    for _,id in ipairs(ids) do
        assert(type(id)=="number" and id>0 and id%1==0,"Invalid loadout ID")
        local key=tostring(id); assert(not result.entries[key],"Duplicate loadout ID")
        local info=C_Traits.GetConfigInfo(id)
        assert(info and type(info.name)=="string" and type(info.usesSharedActionBars)=="boolean","Missing loadout metadata")
        local exported=loadoutExport(id)
        assert(exported.status=="available","Saved loadout content unavailable")
        result.entries[key]={name=info.name,uses_shared_action_bars=info.usesSharedActionBars,import_string=exported.import_string,
            serialization_version=exported.serialization_version}
    end
    return result
end

AO.collectors.talents = function()
    assert(C_ClassTalents and C_ClassTalents.GetActiveConfigID and C_Traits and C_Traits.GetConfigInfo and C_Traits.GetTreeNodes and C_Traits.GetNodeInfo, "Trait API unavailable")
    local configID = C_ClassTalents.GetActiveConfigID()
    assert(configID, "No active config")
    local config = C_Traits.GetConfigInfo(configID)
    assert(config and config.treeIDs, "No active tree")
    local nodes = {}
    for _, treeID in ipairs(config.treeIDs) do
        for _, nodeID in ipairs(C_Traits.GetTreeNodes(treeID)) do
            local node = C_Traits.GetNodeInfo(configID,nodeID)
            if node and node.activeEntry and node.activeEntry.rank > 0 then
                nodes[tostring(nodeID)] = {entry_id=node.activeEntry.entryID, rank=node.activeEntry.rank, tree_id=treeID}
            end
        end
    end
    local result = {config_id=configID, name=config.name, nodes=nodes}
    local exported, loadout = pcall(loadoutExport,configID)
    result.loadout_export = exported and loadout or {status="unavailable", reason="restricted_or_failed"}
    local catalogOK,catalog=pcall(savedLoadouts)
    if catalogOK then result.saved_loadouts=catalog end
    if C_ClassTalents.HasUnspentTalentPoints then
        local has, class, spec = C_ClassTalents.HasUnspentTalentPoints()
        result.has_unspent=has; result.unspent_class=class; result.unspent_spec=spec
    end
    return result
end

AO.collectors.professions = function()
    assert(GetProfessions and GetProfessionInfo, "Profession API unavailable")
    local result = {}
    local ids = {GetProfessions()}
    for slot=1,6 do
        local index=ids[slot]
        if index then
            local name, icon, rank, maxRank, _, _, skillLine = GetProfessionInfo(index)
            result[tostring(slot)]={name=name,rank=rank,max_rank=maxRank,skill_line=skillLine}
        end
    end
    return result
end

AO.collectors.quests = function()
    assert(C_QuestLog and C_QuestLog.GetNumQuestLogEntries and C_QuestLog.GetInfo, "Quest API unavailable")
    local result = {}
    for index=1,C_QuestLog.GetNumQuestLogEntries() do
        local info=C_QuestLog.GetInfo(index)
        if info and not info.isHeader then result[tostring(info.questID)]={title=info.title,level=info.level} end
    end
    return result
end

AO.collectors.currencies = function()
    assert(C_CurrencyInfo and C_CurrencyInfo.GetCurrencyListSize and C_CurrencyInfo.GetCurrencyListInfo, "Currency API unavailable")
    local result = {coverage="visible_currency_list_only", entries={}}
    for index=1,C_CurrencyInfo.GetCurrencyListSize() do
        local info=C_CurrencyInfo.GetCurrencyListInfo(index)
        if info and not info.isHeader then result.entries[tostring(index)]={name=info.name,quantity=info.quantity} end
    end
    return result
end

AO.collectors.achievements = function()
    assert(GetTotalAchievementPoints, "Achievement API unavailable")
    return {coverage="points_only", total_points=GetTotalAchievementPoints()}
end
