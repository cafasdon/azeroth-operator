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
    local stats = { level = UnitLevel("player"), class = select(2, UnitClass("player")), race = select(2, UnitRace("player")), client_version = version }
    if AO.family == "forever" and UnitFactionGroup then
        local ok, faction = pcall(UnitFactionGroup, "player")
        if ok and (not issecretvalue or not issecretvalue(faction))
            and (faction == "Alliance" or faction == "Horde") then
            stats.faction = faction
        end
    end
    return stats
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

AO.collectors.tracked_recipes = function()
    local api=C_TradeSkillUI
    local function accessible(value)
        return not issecretvalue or not issecretvalue(value)
    end
    local function read(isRecraft)
        local ids=api.GetRecipesTracked(isRecraft)
        assert(accessible(ids) and type(ids)=="table" and #ids<=128, "Tracked recipes unavailable")
        local rows,count={},0
        for index,id in pairs(ids) do
            assert(type(index)=="number" and index%1==0 and index>=1 and index<=#ids
                and accessible(id) and type(id)=="number" and id%1==0 and id>=1 and id<=4294967295,
                "Invalid tracked recipe")
            local key=tostring(id)
            assert(not rows[key], "Duplicate tracked recipe")
            local row={}
            local ok,info=pcall(api.GetRecipeInfo,id)
            if ok and accessible(info) and type(info)=="table"
                and accessible(info.recipeID) and info.recipeID==id then
                local name=info.name
                if accessible(name) and type(name)=="string" and #name>0 and #name<=256 then
                    row.name=name
                end
                local learned=info.learned
                if accessible(learned) and type(learned)=="boolean" then row.learned=learned end
            end
            rows[key]=row
            count=count+1
        end
        assert(count==#ids, "Tracked recipe list incomplete")
        return rows
    end
    return {coverage="player_tracked_recipes_only",crafts=read(false),recrafts=read(true)}
end

AO.collectors.selected_recipe = function()
    local page=ProfessionsFrame and ProfessionsFrame.CraftingPage
    local form=page and page.SchematicForm
    assert(form and form:IsVisible() and Professions.InLocalCraftingMode()
        and not form.isInspection and not C_TradeSkillUI.IsNPCCrafting(), "Local recipe is not open")
    local info,schematic=form:GetRecipeInfo(),form.recipeSchematic
    local recipeType=schematic and schematic.recipeType
    local types=Enum.TradeskillRecipeType
    assert(info and schematic and schematic.recipeID==info.recipeID and schematic.isRecraft==false
        and type(recipeType)=="number" and (recipeType==types.Item or recipeType==types.Enchant),
        "Selected recipe is unavailable")
    local function positive(value)
        return type(value)=="number" and value%1==0 and value>=1 and value<=4294967295
    end
    assert(positive(info.recipeID) and type(info.name)=="string" and #info.name>=1 and #info.name<=256
        and type(info.learned)=="boolean", "Recipe identity unavailable")
    local slots=schematic.reagentSlotSchematics
    assert(type(slots)=="table" and #slots<=64, "Recipe reagents unavailable")
    local slotCount=0
    for index in pairs(slots) do
        assert(positive(index) and index<=#slots, "Invalid reagent slot index")
        slotCount=slotCount+1
    end
    assert(slotCount==#slots, "Recipe reagent slots incomplete")
    local result={recipe_id=info.recipeID,name=info.name,learned=info.learned,
        output_item_id=positive(schematic.outputItemID) and schematic.outputItemID or nil,
        requirements="fixed_items",reagents={}}
    local function accessible(value)
        return not issecretvalue or not issecretvalue(value)
    end
    if recipeType==types.Item and result.output_item_id then
        local minimum,maximum=schematic.quantityMin,schematic.quantityMax
        if accessible(minimum) and accessible(maximum) and positive(minimum)
            and positive(maximum) and minimum<=maximum then
            result.base_output_quantity={minimum=minimum,maximum=maximum}
        end
    end
    local difficulty=info.relativeDifficulty
    if accessible(difficulty) and (difficulty==0 or positive(difficulty)) then
        local labels={[0]="optimal","medium","easy","trivial"}
        result.relative_difficulty=labels[difficulty]
    end
    local canSkillUp,skillUps=info.canSkillUp,info.numSkillUps
    if accessible(canSkillUp) and type(canSkillUp)=="boolean" then
        result.can_skill_up=canSkillUp
    end
    if accessible(skillUps) and (skillUps==0 or positive(skillUps)) then
        result.skill_ups_reported=skillUps
    end
    local professionOk,profession=pcall(C_TradeSkillUI.GetProfessionInfoByRecipeID,info.recipeID)
    if professionOk and accessible(profession) and type(profession)=="table" then
        local id,name,rank,cap=profession.professionID,profession.professionName,profession.skillLevel,profession.maxSkillLevel
        if accessible(id) and positive(id) and accessible(name) and type(name)=="string" and #name>0 and #name<=256
            and accessible(rank) and (rank==0 or positive(rank)) and accessible(cap) and (cap==0 or positive(cap)) then
            result.profession={profession_id=id,name=name,rank=rank,max_rank=cap}
        end
    end
    if info.learned then
        local ok,cooldown=pcall(function()
            local seconds,daily,charges,maximum=C_TradeSkillUI.GetRecipeCooldown(info.recipeID)
            local result={}
            if accessible(seconds) and type(seconds)=="number" and seconds>=0 and seconds<=4294967295 then
                result.remaining_seconds=math.ceil(seconds)
                if accessible(daily) and type(daily)=="boolean" then result.daily=daily end
            end
            if accessible(charges) and accessible(maximum) and positive(maximum)
                and (charges==0 or positive(charges)) and charges<=maximum then
                result.charges=charges
                result.max_charges=maximum
            end
            if next(result) then return result end
        end)
        if ok then result.cooldown=cooldown end
    end
    local observed,conditions=pcall(function()
        local rows=C_TradeSkillUI.GetRecipeRequirements(info.recipeID)
        if not accessible(rows) or type(rows)~="table" or #rows>32 then return end
        local labels={[0]="spell_focus","tool","area"}
        local count,conditions=0,{}
        for index,row in pairs(rows) do
            if not positive(index) or index>#rows or not accessible(row) or type(row)~="table" then return end
            local name,met,kind=row.name,row.met,row.type
            if not accessible(name) or not accessible(met) or not accessible(kind)
                or type(name)~="string" or #name==0 or #name>256 or type(met)~="boolean"
                or not (kind==0 or positive(kind)) or not labels[kind] then return end
            conditions[tostring(index)]={name=name,met=met,kind=labels[kind]}
            count=count+1
        end
        if count==#rows then return conditions end
    end)
    if observed then result.crafting_conditions=conditions end
    for _,slot in ipairs(slots) do
        assert(type(slot.required)=="boolean", "Reagent requirement unavailable")
        if slot.required then
            local item=slot.reagents and #slot.reagents==1 and slot.reagents[1]
            if not (item and next(slot.reagents)==1 and next(slot.reagents, 1)==nil
                and positive(item.itemID) and item.currencyID==nil
                and type(slot.variableQuantities)=="table" and next(slot.variableQuantities)==nil
                and positive(slot.quantityRequired)) then
                result.requirements="unsupported_required_slots"
                result.reagents={}
                break
            end
            local id=tostring(item.itemID)
            local count=(result.reagents[id] or 0)+slot.quantityRequired
            assert(positive(count), "Recipe reagent quantity exceeds limit")
            result.reagents[id]=count
        end
    end
    return result
end

local function questObjectives(api,id)
    local rows=api.GetQuestObjectives(id)
    local count=api.GetNumQuestObjectives(id)
    assert(type(rows)=="table" and type(count)=="number" and count>=0 and count<=64 and #rows==count,
        "Quest objectives unavailable")
    local function uint(value,maximum)
        return type(value)=="number" and value%1==0 and value>=0 and value<=maximum
    end
    local result,observed={},0
    for index,row in pairs(rows) do
        assert(uint(index,count) and index>0 and type(row.finished)=="boolean"
            and type(row.type)=="string" and #row.type>=1 and #row.type<=64, "Invalid objective observation")
        local counts=uint(row.numFulfilled,4294967295) and uint(row.numRequired,4294967295)
        result[tostring(index)]={kind=row.type,finished=row.finished,
            fulfilled=counts and row.numFulfilled or nil,required=counts and row.numRequired or nil}
        observed=observed+1
    end
    assert(observed==count, "Quest objectives incomplete")
    return result
end

local function questFlag(api,name,id)
    local ok,value=pcall(api[name],id)
    if ok and (not issecretvalue or not issecretvalue(value)) and type(value)=="boolean" then return value end
end

local function questWaypointMap(api,id)
    local ok,map=pcall(api.GetNextWaypoint,id)
    if not ok or (issecretvalue and issecretvalue(map)) then return nil end
    if type(map)=="number" and map%1==0 and map>=1 and map<=4294967295 then return map end
end

local function questRequiredCopper(api,id)
    local ok,copper=pcall(api.GetRequiredMoney,id)
    if not ok or (issecretvalue and issecretvalue(copper)) then return nil end
    if type(copper)=="number" and copper%1==0 and copper>=0 and copper<=9999999999999 then
        return copper
    end
end

local function questRewardAccessible(value)
    return not issecretvalue or not issecretvalue(value)
end

local function questTimer(api,id)
    local ok,total,elapsed=pcall(api.GetTimeAllowed,id)
    if not ok or not questRewardAccessible(total) or not questRewardAccessible(elapsed) then return nil end
    if type(total)~="number" or type(elapsed)~="number" or not (total>=1 and total<=4294967295)
        or not (elapsed>=0 and elapsed<math.huge) then return nil end
    return {total_seconds=math.floor(total),remaining_seconds=math.floor(math.max(0,total-elapsed))}
end

local function questRewardInteger(value,minimum,maximum)
    return questRewardAccessible(value) and type(value)=="number" and value%1==0
        and value>=minimum and value<=maximum
end

local function questCurrencyRow(row)
    assert(questRewardAccessible(row) and type(row)=="table", "Invalid quest currency row")
    assert(questRewardInteger(row.currencyID,1,4294967295)
        and questRewardInteger(row.totalRewardAmount,1,4294967295), "Invalid quest currency reward")
    return {currency_id=row.currencyID,quantity=row.totalRewardAmount}
end

local function questCurrencyRewards(id)
    local rows=C_QuestInfoSystem.GetQuestRewardCurrencies(id)
    assert(questRewardAccessible(rows) and type(rows)=="table", "Quest currencies unavailable")
    local result,count={},#rows
    assert(count<=32, "Too many quest currencies")
    local observed=0
    for index,row in pairs(rows) do
        assert(questRewardInteger(index,1,count), "Invalid quest currency index")
        result[tostring(index)]=questCurrencyRow(row)
        observed=observed+1
    end
    assert(observed==count, "Incomplete quest currency rewards")
    return result
end

local function questRenownRewards(api,id)
    local show=api.ShouldShowQuestRewards(id)
    if not questRewardAccessible(show) or show~=true then return nil end
    local rows=api.GetQuestLogMajorFactionReputationRewards(id)
    assert(questRewardAccessible(rows) and type(rows)=="table", "Quest renown rewards unavailable")
    local result,count={},#rows
    assert(count<=32, "Too many quest renown rewards")
    local observed=0
    for index,row in pairs(rows) do
        assert(questRewardInteger(index,1,count) and questRewardAccessible(row)
            and type(row)=="table", "Invalid quest renown reward row")
        assert(questRewardInteger(row.factionID,1,4294967295)
            and questRewardInteger(row.rewardAmount,1,4294967295), "Invalid quest renown reward")
        result[tostring(index)]={faction_id=row.factionID,quantity=row.rewardAmount}
        observed=observed+1
    end
    assert(observed==count, "Incomplete quest renown rewards")
    return result
end

local function questSpellRewards(id)
    local api=C_QuestInfoSystem
    local rows=api.GetQuestRewardSpells(id)
    assert(questRewardAccessible(rows) and type(rows)=="table", "Quest spell rewards unavailable")
    local result,count={},#rows
    assert(count<=32, "Too many quest spell rewards")
    local observed=0
    for index,spellID in pairs(rows) do
        assert(questRewardInteger(index,1,count) and questRewardInteger(spellID,1,4294967295),
            "Invalid quest spell reward")
        local entry={spell_id=spellID}
        local nameOk,name=pcall(C_Spell and C_Spell.GetSpellName,spellID)
        if nameOk and questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256 then
            entry.name=name
        end
        local bank=Enum and Enum.SpellBookSpellBank and Enum.SpellBookSpellBank.Player
        if questRewardInteger(bank,0,4294967295) then
            local bookOk,listed=pcall(C_SpellBook and C_SpellBook.IsSpellInSpellBook,spellID,bank,true)
            if bookOk and questRewardAccessible(listed) and type(listed)=="boolean" then
                entry.in_player_spellbook=listed
            end
        end
        local ok,info=pcall(api.GetQuestRewardSpellInfo,id,spellID)
        if ok and questRewardAccessible(info) and type(info)=="table" then
            local kind,learned=info.type,info.isSpellLearned
            if questRewardInteger(kind,0,4294967295) then entry.reward_type=kind end
            if questRewardAccessible(learned) and type(learned)=="boolean" then
                entry.is_spell_learned=learned
            end
        end
        result[tostring(index)]=entry
        observed=observed+1
    end
    assert(observed==count, "Incomplete quest spell rewards")
    return result
end

local function questItemReward(query,index)
    local _,_,quantity,_,_,itemID=query(index)
    assert(questRewardInteger(itemID,1,4294967295)
        and questRewardInteger(quantity,1,4294967295), "Invalid quest item reward")
    return {item_id=itemID,quantity=quantity}
end

local function questFixedItems()
    local count=GetNumQuestLogRewards()
    assert(questRewardInteger(count,0,32), "Invalid quest item reward count")
    local result={}
    for index=1,count do result[tostring(index)]=questItemReward(GetQuestLogRewardInfo,index) end
    return result
end

local function questRewardAmount(query,maximum)
    local ok,value=pcall(query)
    if ok and questRewardInteger(value,0,maximum) then return value end
end

local function questSelectedRewards(api,id)
    local selected=api.GetSelectedQuest()
    if not questRewardInteger(selected,1,4294967295) or selected~=id then return nil end
    local show=api.ShouldShowQuestRewards(id)
    if not questRewardAccessible(show) or show~=true then return nil end
    local fixedOk,fixed=pcall(questFixedItems)
    local amounts={
        copper=questRewardAmount(GetQuestLogRewardMoney,9999999999999),
        xp=questRewardAmount(GetQuestLogRewardXP,4294967295),
        honor=questRewardAmount(GetQuestLogRewardHonor,4294967295),
    }
    local choicesOk,currencies,items=pcall(function()
        local count=GetNumQuestLogChoices(id,true)
        assert(questRewardInteger(count,0,32), "Invalid quest choice count")
        local currencies,items={},{}
        for index=1,count do
            local kind=GetQuestLogChoiceInfoLootType(index)
            assert(questRewardInteger(kind,0,1), "Unknown quest choice type")
            if kind==1 and currencies then
                local ok,row=pcall(function() return questCurrencyRow(api.GetQuestRewardCurrencyInfo(id,index,true)) end)
                if ok then currencies[tostring(index)]=row else currencies=nil end
            elseif kind==0 and items then
                local ok,row=pcall(questItemReward,GetQuestLogChoiceInfo,index)
                if ok then items[tostring(index)]=row else items=nil end
            end
        end
        return currencies,items
    end)
    selected=api.GetSelectedQuest()
    if questRewardInteger(selected,1,4294967295) and selected==id then
        return choicesOk and currencies or nil,choicesOk and items or nil,fixedOk and fixed or nil,
            next(amounts) and amounts or nil
    end
end

AO.collectors.quests = function()
    assert(C_QuestLog and C_QuestLog.GetNumQuestLogEntries and C_QuestLog.GetInfo, "Quest API unavailable")
    local result = {}
    local api=C_QuestLog
    local count=api.GetNumQuestLogEntries()
    if AO.family=="forever" then
        assert(type(count)=="number" and count%1==0 and count>=0 and count<=512, "Invalid quest log size")
    end
    for index=1,count do
        local info=api.GetInfo(index)
        if AO.family=="forever" then
            assert(info and type(info.isHeader)=="boolean", "Quest log row unavailable")
        end
        if info and not info.isHeader then
            local id=info.questID
            local entry={title=info.title,level=info.level}
            if AO.family=="forever" then
                assert(type(id)=="number" and id%1==0 and id>=1 and id<=4294967295
                    and not result[tostring(id)], "Invalid or duplicate quest ID")
                entry.ready_for_turn_in=questFlag(api,"ReadyForTurnIn",id)
                entry.failed=questFlag(api,"IsFailed",id)
                entry.next_waypoint_map_id=questWaypointMap(api,id)
                entry.required_copper=questRequiredCopper(api,id)
                entry.timer=questTimer(api,id)
                entry.elite=questFlag(api,"IsEliteQuest",id)
                local groupOk,group=pcall(api.GetSuggestedGroupSize,id)
                if groupOk and questRewardInteger(group,0,4294967295) then entry.suggested_group_size=group end
                local rewardsOk,rewards=pcall(questCurrencyRewards,id)
                if rewardsOk then entry.currency_rewards=rewards end
                local renownOk,renown=pcall(questRenownRewards,api,id)
                if renownOk then entry.renown_rewards=renown end
                local spellsOk,spells=pcall(questSpellRewards,id)
                if spellsOk then entry.spell_rewards=spells end
                local selectedOk,currencyChoices,itemChoices,itemRewards,rewardAmounts=pcall(questSelectedRewards,api,id)
                if selectedOk then
                    entry.currency_choices=currencyChoices
                    entry.item_choices=itemChoices
                    entry.item_rewards=itemRewards
                    entry.reward_amounts=rewardAmounts
                end
                local ok,objectives=pcall(questObjectives,api,id)
                if ok then entry.objectives=objectives end
            end
            result[tostring(id)]=entry
        end
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

AO.collectors.spellbook = function()
    local api=C_SpellBook
    local bank,spellType=Enum.SpellBookSpellBank.Player,Enum.SpellBookItemType.Spell
    assert(questRewardInteger(bank,0,4294967295) and questRewardInteger(spellType,0,4294967295), "Spellbook API unavailable")
    local count=api.GetNumSpellBookSkillLines()
    assert(questRewardInteger(count,0,64), "Invalid spellbook skill line count")
    local entries,visited,total={},{},0
    for line=1,count do
        local skill=api.GetSpellBookSkillLineInfo(line)
        assert(questRewardAccessible(skill) and type(skill)=="table", "Spellbook skill line unavailable")
        local hidden,guild=skill.shouldHide,skill.isGuild
        assert(questRewardAccessible(hidden) and type(hidden)=="boolean"
            and questRewardAccessible(guild) and type(guild)=="boolean", "Invalid spellbook skill line")
        if not hidden and not guild then
            local offset,length=skill.itemIndexOffset,skill.numSpellBookItems
            assert(questRewardInteger(offset,0,4096) and questRewardInteger(length,0,1024)
                and offset+length<=4096 and total+length<=1024, "Invalid spellbook range")
            total=total+length
            for slot=offset+1,offset+length do
                assert(not visited[slot], "Overlapping spellbook ranges")
                visited[slot]=true
                local item=api.GetSpellBookItemInfo(slot,bank)
                assert(questRewardAccessible(item) and type(item)=="table"
                    and questRewardInteger(item.itemType,0,4294967295), "Spellbook entry unavailable")
                if item.itemType==spellType then
                    assert(questRewardInteger(item.actionID,1,4294967295)
                        and questRewardInteger(item.spellID,1,4294967295), "Invalid spellbook spell")
                    entries[tostring(slot)]={base_spell_id=item.actionID,spell_id=item.spellID}
                end
            end
        end
    end
    return {coverage="visible_player_spell_slots",entries=entries}
end

AO.collectors.experience = function()
    local function accessible(value)
        return not issecretvalue or not issecretvalue(value)
    end
    local function uint(value)
        return accessible(value) and type(value)=="number" and value%1==0 and value>=0 and value<=4294967295
    end
    local trial=GameLimitedMode_IsBankedXPActive()
    assert(accessible(trial) and type(trial)=="boolean", "Trial XP status unavailable")
    if trial then
        local level,limit=UnitLevel("player"),GameLimitedMode_GetLevelLimit()
        assert(uint(level) and uint(limit) and limit>0, "Trial XP level unavailable")
        if level>=limit then return {status="trial_capped"} end
    end
    local capped=GameRulesUtil.IsPlayerAtEffectiveMaxLevel()
    assert(accessible(capped) and type(capped)=="boolean", "XP cap status unavailable")
    if capped then return {status="level_capped"} end
    local current,threshold=UnitXP("player"),UnitXPMax("player")
    assert(uint(current) and uint(threshold) and threshold>0 and current<=threshold, "Invalid XP observation")
    local result={status="progress",current_xp=current,level_threshold_xp=threshold}
    local ok,disabled=pcall(IsXPUserDisabled)
    if ok and accessible(disabled) and type(disabled)=="boolean" then result.xp_disabled=disabled end
    local restedOk,rested=pcall(GetXPExhaustion)
    if restedOk and uint(rested) then result.rested_xp=rested end
    return result
end

AO.collectors.wallet = function()
    local copper = GetMoney()
    assert(not issecretvalue or not issecretvalue(copper), "Wallet value restricted")
    assert(type(copper)=="number" and copper>=0 and copper<=9999999999999
        and copper==math.floor(copper), "Invalid wallet balance")
    return {copper=copper}
end

AO.collectors.currency_balances = function()
    local api=C_CurrencyInfo
    assert(api and api.GetCurrencyListSize and api.GetCurrencyListInfo, "Currency API unavailable")
    local function uint(value, maximum)
        return type(value)=="number" and value%1==0 and value>=0 and value<=maximum
    end
    local count=api.GetCurrencyListSize()
    assert(uint(count,512) and count>0, "Currency rows unavailable")
    local rows={}
    for index=1,count do
        local row=api.GetCurrencyListInfo(index)
        assert(row and type(row.isHeader)=="boolean", "Currency row unavailable")
        if not row.isHeader then
            assert(uint(row.currencyID,4294967295) and row.currencyID>0
                and type(row.name)=="string" and #row.name>=1 and #row.name<=256
                and type(row.isAccountWide)=="boolean" and type(row.useTotalEarnedForMaxQty)=="boolean"
                and type(row.isTypeUnused)=="boolean", "Currency identity or scope unavailable")
            for _,field in ipairs({"quantity","maxQuantity","maxWeeklyQuantity","quantityEarnedThisWeek","totalEarned"}) do
                assert(uint(row[field],4294967295), "Currency quantity unavailable")
            end
            local id=tostring(row.currencyID)
            assert(not rows[id], "Duplicate currency observation")
            rows[id]={name=row.name,quantity=row.quantity,account_wide=row.isAccountWide,inactive=row.isTypeUnused,
                max_quantity=row.maxQuantity,max_weekly_quantity=row.maxWeeklyQuantity,
                earned_this_week=row.quantityEarnedThisWeek,total_earned=row.totalEarned,
                cap_uses_total_earned=row.useTotalEarnedForMaxQty}
            local weekly=row.canEarnPerWeek
            if (not issecretvalue or not issecretvalue(weekly)) and type(weekly)=="boolean" then
                rows[id].can_earn_per_week=weekly
            end
        end
    end
    assert(api.GetCurrencyListSize()==count, "Currency rows changed")
    return {coverage="visible_currency_ids_only",entries=rows}
end

AO.collectors.achievements = function()
    assert(GetTotalAchievementPoints, "Achievement API unavailable")
    return {coverage="points_only", total_points=GetTotalAchievementPoints()}
end

AO.collectors.tracked_achievements = function()
    local function uint(value, maximum)
        return type(value)=="number" and value%1==0 and value>=0 and value<=maximum
    end
    local ids=C_ContentTracking.GetTrackedIDs(Enum.ContentTrackingType.Achievement)
    assert(type(ids)=="table" and #ids<=32, "Tracked achievement list unavailable")
    local count=0
    for index in pairs(ids) do
        assert(uint(index,#ids) and index>0, "Invalid tracked achievement index")
        count=count+1
    end
    assert(count==#ids, "Tracked achievement list incomplete")
    local seen,rows={},{}
    for _,id in ipairs(ids) do
        assert(uint(id,4294967295) and id>0 and not seen[id], "Invalid tracked achievement ID")
        seen[id]=true
        local observedID,name,points,completed,_,_,_,_,flags,_,_,isGuild,earnedByCharacter=GetAchievementInfo(id)
        assert(observedID==id and type(isGuild)=="boolean", "Tracked achievement unavailable")
        if not isGuild then
            assert(type(name)=="string" and #name>=1 and #name<=256 and uint(points,4294967295)
                and type(completed)=="boolean" and type(earnedByCharacter)=="boolean"
                and uint(flags,4294967295) and uint(ACHIEVEMENT_FLAGS_ACCOUNT,4294967295)
                and ACHIEVEMENT_FLAGS_ACCOUNT>0, "Invalid achievement observation")
            local criteriaCount=GetAchievementNumCriteria(id)
            assert(uint(criteriaCount,128), "Achievement criteria unavailable")
            local criteria={}
            for index=1,criteriaCount do
                local _,_,done,quantity,required,_,_,_,_,criteriaID=GetAchievementCriteriaInfo(id,index)
                assert(type(done)=="boolean", "Achievement criterion unavailable")
                local quantitiesValid=uint(quantity,4294967295) and uint(required,4294967295)
                criteria[tostring(index)]={completed=done,
                    criterion_id=uint(criteriaID,4294967295) and criteriaID>0 and criteriaID or nil,
                    quantity=quantitiesValid and quantity or nil,required=quantitiesValid and required or nil}
            end
            rows[tostring(id)]={name=name,points=points,completed=completed,earned_by_character=earnedByCharacter,
                account_wide=bit.band(flags,ACHIEVEMENT_FLAGS_ACCOUNT)==ACHIEVEMENT_FLAGS_ACCOUNT,criteria=criteria}
        end
    end
    return {coverage="tracked_player_achievements_only",entries=rows}
end

AO.collectors.reputation = function()
    local api=C_Reputation
    assert(api and api.GetNumFactions and api.GetFactionDataByIndex, "Reputation API unavailable")
    local function integer(value, low, high)
        return type(value)=="number" and value%1==0 and value>=low and value<=high
    end
    local function accessible(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted reputation observation")
        return value
    end
    local count=api.GetNumFactions()
    assert(integer(count,1,512), "Reputation rows unavailable")
    local rows={}
    for index=1,count do
        local row=api.GetFactionDataByIndex(index)
        assert(row and type(row.isHeader)=="boolean" and type(row.isHeaderWithRep)=="boolean",
            "Reputation row unavailable")
        if not row.isHeader or row.isHeaderWithRep then
            assert(integer(row.factionID,1,4294967295) and type(row.name)=="string" and #row.name>=1
                and #row.name<=256 and type(row.isAccountWide)=="boolean"
                and integer(row.reaction,1,8) and integer(row.currentStanding,-2147483648,2147483647)
                and integer(row.currentReactionThreshold,-2147483648,2147483647)
                and integer(row.nextReactionThreshold,-2147483648,2147483647), "Invalid faction observation")
            local id=tostring(row.factionID)
            assert(not rows[id], "Duplicate faction observation")
            local ok,kind,friendship=pcall(function()
                local major=api.IsMajorFaction(row.factionID)
                local paragon=api.IsFactionParagonForCurrentPlayer(row.factionID)
                local friendship=accessible(C_GossipInfo.GetFriendshipReputation(row.factionID))
                local friendshipID=friendship and accessible(friendship.friendshipFactionID)
                if type(major)=="boolean" and type(paragon)=="boolean"
                    and type(friendshipID)=="number" then
                    if paragon then return "paragon" end
                    if major then return "renown" end
                    if friendshipID>0 then return "friendship",friendship end
                    if friendshipID==0 then return "standard" end
                end
            end)
            rows[id]={name=row.name,kind=ok and kind or "unknown",account_wide=row.isAccountWide,reaction=row.reaction,
                standing=row.currentStanding,threshold=row.currentReactionThreshold,next_threshold=row.nextReactionThreshold}
            if rows[id].kind=="renown" then
                local observed,progress=pcall(function()
                    local data=accessible(C_MajorFactions.GetMajorFactionData(row.factionID))
                    local capped=accessible(C_MajorFactions.HasMaximumRenown(row.factionID))
                    if type(data)~="table" or type(capped)~="boolean" then return end
                    local level=accessible(data.renownLevel)
                    local earned=accessible(data.renownReputationEarned)
                    local threshold=accessible(data.renownLevelThreshold)
                    if integer(level,0,4294967295) and integer(earned,0,4294967295)
                        and integer(threshold,0,4294967295) then
                        return {level=level,earned_in_level=earned,level_threshold=threshold,capped=capped}
                    end
                end)
                if observed then rows[id].renown=progress end
            elseif rows[id].kind=="friendship" then
                local observed,progress=pcall(function()
                    local standing=accessible(friendship.standing)
                    local threshold=accessible(friendship.reactionThreshold)
                    local nextThreshold=accessible(friendship.nextThreshold)
                    local function points(value)
                        return integer(value,-2147483648,2147483647)
                    end
                    if points(standing) and points(threshold)
                        and (nextThreshold==nil or points(nextThreshold)) then
                        return {standing=standing,threshold=threshold,next_threshold=nextThreshold}
                    end
                end)
                if observed then rows[id].friendship=progress end
            elseif rows[id].kind=="paragon" then
                local observed,progress=pcall(function()
                    local current,threshold,quest,pending,tooLow=api.GetFactionParagonInfo(row.factionID)
                    current,threshold=accessible(current),accessible(threshold)
                    pending,tooLow=accessible(pending),accessible(tooLow)
                    if integer(current,0,4294967295) and integer(threshold,1,4294967295)
                        and type(pending)=="boolean" and type(tooLow)=="boolean" then
                        local result={current_value=current,reward_threshold=threshold,
                            reward_pending=pending,level_too_low=tooLow}
                        if (not issecretvalue or not issecretvalue(quest)) and integer(quest,1,4294967295) then
                            result.reward_quest_id=quest
                        end
                        return result
                    end
                end)
                if observed then rows[id].paragon=progress end
            end
        end
    end
    assert(api.GetNumFactions()==count, "Reputation rows changed")
    return {coverage="visible_faction_rows_only",entries=rows}
end
