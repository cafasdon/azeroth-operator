local _, AO = ...
AO.collectors = {}
local function itemGUID(bag,slot)
    if AO.family~="retail" or not (C_Item and C_Item.GetItemGUID and ItemLocation) then return nil end
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

AO.collectors.bags = function()
    local bags = {}
    if C_Container and C_Container.GetContainerNumSlots and C_Container.GetContainerItemInfo then
        for bag = 0, (NUM_TOTAL_EQUIPPED_BAG_SLOTS or NUM_BAG_SLOTS or 4) do
            local slots = {}
            for slot = 1, C_Container.GetContainerNumSlots(bag) do
                local info = C_Container.GetContainerItemInfo(bag, slot)
                if info then slots[tostring(slot)] = { item_id = info.itemID, count = info.stackCount, link = info.hyperlink, guid=itemGUID(bag,slot) } end
            end
            bags[tostring(bag)] = slots
        end
        return bags
    end
    error("Container API unavailable")
end

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
