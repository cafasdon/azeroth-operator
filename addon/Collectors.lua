local _, AO = ...
AO.collectors = {}
-- Observe only the already-open loot panel during a manual export.
AO.collectors.loot_window = function()
    assert(AO.family=="forever" and LootFrame and type(LootFrame.IsShown)=="function"
        and type(GetNumLootItems)=="function" and type(GetLootSlotLink)=="function"
        and type(GetLootSlotInfo)=="function", "Loot window unavailable")
    local frame=LootFrame
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted loot observation")
        return value
    end
    local function integer(value,low,high)
        value=public(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high,
            "Invalid loot number")
        return value
    end
    local function context()
        local shown=public(frame:IsShown())
        assert(LootFrame==frame and type(shown)=="boolean" and shown, "Loot window is not shown")
        return integer(GetNumLootItems(),0,64)
    end
    local function read(slot)
        local link=public(GetLootSlotLink(slot))
        assert(type(link)=="string" and #link<=2048, "Unreadable loot link")
        -- Accept only a complete public item hyperlink, with an optional colour wrapper.
        local id=link:match("^|Hitem:([1-9]%d*):[^|]*|h[^|]*|h$")
            or link:match("^|c%x%x%x%x%x%x%x%x|Hitem:([1-9]%d*):[^|]*|h[^|]*|h|r$")
        assert(id and #id<=10, "Unreadable loot item")
        id=integer(tonumber(id),1,4294967295)
        local _,_,quantity=GetLootSlotInfo(slot)
        return {status="item",item_id=id,quantity=integer(quantity,0,4294967295)}
    end
    local count=context()
    local first={}
    for slot=1,count do
        local ok,row=pcall(read,slot)
        if ok then first[slot]=row end
    end
    assert(context()==count, "Loot window changed")
    assert(context()==count, "Loot window changed")
    local entries={}
    for slot=1,count do
        local ok,row=pcall(read,slot)
        local prior=first[slot]
        entries[tostring(slot)]=ok and prior and prior.item_id==row.item_id
            and prior.quantity==row.quantity and row or {status="unreadable_or_non_item"}
    end
    assert(context()==count, "Loot window changed")
    return {coverage="visible_loot_window_slots",slot_count=count,entries=entries}
end
-- Optional separately installed reference provider; no provider code/data is bundled.
AO.collectors.recipe_item_sources = function(materials)
    assert(AO.family=="forever" and type(materials)=="table", "Recipe materials unavailable")
    local provider=LibQuestieDB
    assert(type(provider)=="table" and type(provider.RequireContract)=="function"
        and type(provider.ModeIndicator)=="table" and type(provider.ModeIndicator.GetStatus)=="function"
        and type(provider.Item)=="table" and type(provider.Item.Exists)=="function"
        and type(provider.Item.Get)=="function", "Reference provider unavailable")
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted reference value")
        return value
    end
    local function uint(value,low,high)
        value=public(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid reference number")
        return value
    end
    local function context()
        assert(public(provider.RequireContract(1))==true, "Reference contract unavailable")
        local status=public(provider.ModeIndicator.GetStatus())
        assert(type(status)=="table" and public(status.expansion)=="Forever", "Reference flavour unavailable")
        local mode=public(status.mode)
        assert(mode=="source" or mode=="baked", "Reference mode unavailable")
        return mode,uint(status.contractVersion,1,4294967295)
    end
    local mode,contract=context()
    local ids,seen={},{}
    for _,row in pairs(materials.entries) do
        if row.status=="fixed_items" then
            for id in pairs(row.reagents) do
                id=uint(tonumber(id),1,4294967295)
                if not seen[id] then seen[id]=true; ids[#ids+1]=id end
            end
        end
    end
    assert(#ids<=16384, "Reference input budget exceeded")
    table.sort(ids)
    local fields={npc_drops="npcDrops",object_drops="objectDrops",container_drops="itemDrops",
        quest_rewards="questRewards",vendors="vendors"}
    local function references(id,key)
        local values=public(provider.Item.Get(id,key))
        if values==nil then return end -- Provider normalises empty arrays to nil; not proof of absence.
        assert(type(values)=="table", "Invalid reference list")
        local unique,count,maximum,positions={},0,0,0
        for position,value in pairs(values) do
            position=uint(position,1,4096); positions=positions+1
            assert(positions<=4096, "Reference list budget exceeded")
            maximum=math.max(maximum,position)
            value=uint(value,1,4294967295)
            if not unique[value] then unique[value]=true; count=count+1 end
        end
        assert(maximum==positions, "Sparse reference list")
        local ordered={}; for value in pairs(unique) do ordered[#ordered+1]=value end
        table.sort(ordered)
        local entries={}; for i=1,math.min(8,#ordered) do entries[tostring(ordered[i])]=true end
        return {entries=entries,distinct_id_count=count,truncated=count>8}
    end
    local function read(id)
        local exists=public(provider.Item.Exists(id))
        assert(type(exists)=="boolean", "Invalid reference existence")
        if not exists then return {status="not_in_provider"} end
        local row={status="reported"}
        for field,key in pairs(fields) do
            local ok,value=pcall(references,id,key)
            if ok then row[field]=value end
        end
        return row
    end
    local function equal(a,b)
        if type(a)~=type(b) then return false end
        if type(a)~="table" then return a==b end
        for key,value in pairs(a) do if not equal(value,b[key]) then return false end end
        for key in pairs(b) do if a[key]==nil then return false end end
        return true
    end
    local entries={}
    for i=1,math.min(64,#ids) do
        local ok,first=pcall(read,ids[i]); local again,second=pcall(read,ids[i])
        entries[tostring(ids[i])]=ok and again and equal(first,second) and first or {status="unavailable"}
    end
    -- These are provider area percentages, never native map coordinates or visibility.
    local function entityApi(name)
        local namespace=public(provider[name])
        assert(type(namespace)=="table", "Reference entity namespace unavailable")
        local exists=public(namespace.Exists); local get=public(namespace.Get)
        assert(type(exists)=="function" and type(get)=="function", "Reference entity API unavailable")
        return exists,get
    end
    local function spawns(get,id)
        local areas=public(get(id,"spawns"))
        if areas==nil then return end
        assert(type(areas)=="table", "Invalid reference spawns")
        local points,areaCount={},0
        for area,positions in pairs(areas) do
            areaCount=areaCount+1
            assert(areaCount<=4096, "Reference area budget exceeded")
            area=uint(area,1,4294967295); positions=public(positions)
            assert(type(positions)=="table", "Invalid reference spawn positions")
            local count,maximum=0,0
            for position,tuple in pairs(positions) do
                position=uint(position,1,4096); tuple=public(tuple)
                count=count+1; maximum=math.max(maximum,position)
                assert(type(tuple)=="table" and #points<4096, "Reference spawn budget exceeded")
                local values,fields={},0
                for index,value in pairs(tuple) do
                    index=uint(index,1,3); value=public(value)
                    assert(type(value)=="number", "Invalid reference spawn value")
                    values[index]=value; fields=fields+1
                end
                assert((fields==2 or fields==3) and values[1]~=nil and values[2]~=nil
                    and (fields==2 or values[3]~=nil), "Invalid reference spawn tuple")
                local x,y,phase=values[1],values[2],values[3]
                local point={area_id=area}
                if x==-1 and y==-1 then
                    assert(phase==nil, "Invalid reference instance presence")
                    point.kind="instance_presence"
                else
                    assert(x>=0 and x<=100 and y>=0 and y<=100, "Invalid reference coordinates")
                    point.kind="coordinate"; point.x_percent=x; point.y_percent=y
                    if phase~=nil then
                        phase=uint(phase,0,4294967295)
                        if phase>0 then point.provider_phase_id=phase end
                    end
                end
                points[#points+1]=point
            end
            assert(count==maximum, "Sparse reference spawn positions")
        end
        table.sort(points,function(a,b)
            if a.area_id~=b.area_id then return a.area_id<b.area_id end
            if a.kind~=b.kind then return a.kind<b.kind end
            if a.x_percent~=b.x_percent then return a.x_percent<b.x_percent end
            if a.y_percent~=b.y_percent then return a.y_percent<b.y_percent end
            return (a.provider_phase_id or 0)<(b.provider_phase_id or 0)
        end)
        local captured={}
        for i=1,math.min(16,#points) do captured[tostring(i)]=points[i] end
        return {reported_point_count=#points,truncated=#points>16,entries=captured}
    end
    local function entityRead(name,id)
        local exists,get=entityApi(name)
        local found=public(exists(id))
        assert(type(found)=="boolean", "Invalid reference entity existence")
        if not found then return {status="not_in_provider"} end
        local row={status="reported"}
        local ok,value=pcall(spawns,get,id)
        if ok then row.spawns=value end
        return row
    end
    local entityLocations
    local npcAvailable=pcall(entityApi,"Npc")
    local objectAvailable=pcall(entityApi,"Object")
    if npcAvailable or objectAvailable then
        local npcSet,objectSet={},{}
        for _,row in pairs(entries) do
            if row.status=="reported" then
                for _,field in ipairs({"npc_drops","vendors","object_drops"}) do
                    if row[field] then
                        for id in pairs(row[field].entries) do
                            local target=field=="object_drops" and objectSet or npcSet
                            target[tonumber(id)]=true
                        end
                    end
                end
            end
        end
        local function captureEntities(name,set,budget)
            local ordered={}; for id in pairs(set) do ordered[#ordered+1]=id end
            assert(#ordered<=budget, "Reference entity budget exceeded")
            table.sort(ordered)
            local captured={}
            for i=1,math.min(32,#ordered) do
                local ok,first=pcall(entityRead,name,ordered[i])
                local again,second=pcall(entityRead,name,ordered[i])
                captured[tostring(ordered[i])]=ok and again and equal(first,second)
                    and first or {status="unavailable"}
            end
            return captured,#ordered
        end
        local npcs,npcCount=captureEntities("Npc",npcSet,1024)
        local objects,objectCount=captureEntities("Object",objectSet,512)
        entityLocations={coordinate_system="questiedb_area_percent",reported_npc_count=npcCount,
            reported_object_count=objectCount,npcs=npcs,objects=objects}
    end
    local afterMode,afterContract=context()
    assert(LibQuestieDB==provider and afterMode==mode and afterContract==contract, "Reference context changed")
    return {coverage="first_reported_fixed_recipe_reagents",provider="questiedb",provider_mode=mode,
        provider_contract_version=contract,profession_id=materials.profession_id,
        reported_item_count=#ids,entries=entries,entity_locations=entityLocations}
end
local savedInstanceUpdate
if AO.family=="forever" and CreateFrame then
    local ok,observer=pcall(function()
        local frame=CreateFrame("Frame")
        frame:RegisterEvent("UPDATE_INSTANCE_INFO")
        frame:RegisterEvent("PLAYER_ENTERING_WORLD")
        frame:SetScript("OnEvent",function(_,event)
            savedInstanceUpdate=nil
            if event=="UPDATE_INSTANCE_INFO" and GetServerTime then
                local success,value=pcall(GetServerTime)
                if success and (not issecretvalue or not issecretvalue(value))
                    and type(value)=="number" and value%1==0 and value>0 and value<=4294967295 then
                    savedInstanceUpdate=value
                end
            end
        end)
        return frame
    end)
    if ok then AO.savedInstanceObserver=observer end
end
AO.collectors.saved_world_bosses = function()
    assert(AO.family=="forever" and savedInstanceUpdate and GetServerTime
        and GetNumSavedWorldBosses and GetSavedWorldBossInfo, "Saved world boss cache unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted saved world boss observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high,
            "Invalid saved world boss number")
        return value
    end
    local updated=savedInstanceUpdate
    local now=integer(GetServerTime(),updated,4294967295)
    local count=integer(GetNumSavedWorldBosses(),0,128)
    local result={coverage="updated_cache",update_age_seconds=now-updated,entries={}}
    for index=1,count do
        local name,_,reset=GetSavedWorldBossInfo(index)
        name=readable(name); reset=readable(reset)
        assert(type(name)=="string" and #name>0 and #name<=256, "Invalid saved world boss name")
        assert(type(reset)=="number", "Invalid saved world boss timer")
        result.entries[tostring(index)]={name=name,
            reset_seconds=integer(math.ceil(reset),-2147483648,2147483647)}
    end
    assert(integer(GetNumSavedWorldBosses(),0,128)==count and savedInstanceUpdate==updated,
        "Saved world boss cache changed")
    return result
end
AO.collectors.saved_instances = function()
    assert(AO.family=="forever" and savedInstanceUpdate and GetServerTime
        and GetNumSavedInstances and GetSavedInstanceInfo, "Saved instance cache unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted saved instance observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid saved instance number")
        return value
    end
    local function boolean(value)
        value=readable(value); assert(type(value)=="boolean", "Invalid saved instance flag"); return value
    end
    local function text(value)
        value=readable(value)
        assert(type(value)=="string" and #value>0 and #value<=256, "Invalid saved instance name")
        return value
    end
    local updated=savedInstanceUpdate
    local now=integer(GetServerTime(),updated,4294967295)
    local count=integer(GetNumSavedInstances(),0,128)
    local result={coverage="updated_cache",update_age_seconds=now-updated,entries={}}
    for index=1,count do
        local name,_,reset,difficulty,locked,extended,_,raid,players,difficultyName=GetSavedInstanceInfo(index)
        reset=readable(reset)
        assert(type(reset)=="number", "Invalid saved instance timer")
        local row={name=text(name),reset_seconds=integer(math.ceil(reset),-2147483648,2147483647),
            difficulty_id=integer(difficulty,0,4294967295),locked=boolean(locked),extended=boolean(extended),
            is_raid=boolean(raid),max_players=integer(players,0,4294967295)}
        difficultyName=readable(difficultyName)
        if difficultyName~=nil then row.difficulty_name=text(difficultyName) end
        result.entries[tostring(index)]=row
    end
    assert(integer(GetNumSavedInstances(),0,128)==count and savedInstanceUpdate==updated,
        "Saved instance cache changed")
    return result
end
AO.collectors.toy_collection_view = function()
    local api=C_ToyBox
    assert(AO.family=="forever" and ToyBox and ToyBox.IsVisible and api
        and api.GetNumFilteredToys and api.GetToyFromIndex and api.GetToyInfo and PlayerHasToy,
        "Toy journal unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted toy observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid toy identifier")
        return value
    end
    local function boolean(value)
        value=readable(value); assert(type(value)=="boolean", "Invalid toy flag"); return value
    end
    assert(boolean(ToyBox:IsVisible()), "Toy journal is not visible")
    local count=integer(api.GetNumFilteredToys(),0,4096)
    local result={coverage="filtered_journal",displayed_count=count,entries={}}
    local ids={}
    for index=1,count do
        local id=integer(api.GetToyFromIndex(index),1,4294967295)
        local key=tostring(id)
        assert(not result.entries[key], "Duplicate toy ID")
        local returnedID,name,_,favorite=api.GetToyInfo(id)
        assert(integer(returnedID,1,4294967295)==id, "Toy identity changed")
        name=readable(name)
        assert(type(name)=="string" and #name>0 and #name<=256, "Invalid toy name")
        result.entries[key]={name=name,collected=boolean(PlayerHasToy(id)),favorite=boolean(favorite)}
        ids[index]=id
    end
    assert(boolean(ToyBox:IsVisible()) and integer(api.GetNumFilteredToys(),0,4096)==count,
        "Toy view changed")
    for index=1,count do
        assert(integer(api.GetToyFromIndex(index),1,4294967295)==ids[index], "Toy view changed")
    end
    return result
end
AO.collectors.mount_collection = function()
    local api=C_MountJournal
    assert(AO.family=="forever" and api and api.GetMountIDs and api.GetMountInfoByID,
        "Mount journal unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted mount observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high,
            "Invalid mount identifier")
        return value
    end
    local function boolean(value)
        value=readable(value); assert(type(value)=="boolean", "Invalid mount flag"); return value
    end
    local function catalogue()
        local ids=readable(api.GetMountIDs())
        assert(type(ids)=="table", "Missing mount catalogue")
        local result,indices,count={},{},0
        for index,id in pairs(ids) do
            index=integer(index,1,4096); id=integer(id,1,4294967295)
            assert(not result[tostring(id)], "Duplicate mount ID")
            result[tostring(id)]=id; indices[index]=true; count=count+1
        end
        assert(count>0 and count<=4096, "Empty or oversized mount catalogue")
        for index=1,count do assert(indices[index], "Incomplete mount catalogue") end
        return result,count
    end
    local ids,count=catalogue()
    local result={catalogue_count=count,entries={}}
    for key,id in pairs(ids) do
        local name,spellID,_,_,usable,_,favorite,_,_,_,collected,returnedID=api.GetMountInfoByID(id)
        assert(integer(returnedID,1,4294967295)==id, "Mount identity changed")
        if boolean(collected) then
            name=readable(name)
            assert(type(name)=="string" and #name>0 and #name<=256, "Invalid mount name")
            result.entries[key]={name=name,spell_id=integer(spellID,1,4294967295),
                usable=boolean(usable),favorite=boolean(favorite)}
        end
    end
    local after,afterCount=catalogue()
    assert(afterCount==count, "Mount catalogue changed")
    for key in pairs(ids) do assert(after[key], "Mount catalogue changed") end
    return result
end
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

local function itemNativeStats(item)
    if AO.family~="forever" or not (C_Item and C_Item.GetItemStats and C_Item.GetItemInfoInstant) then return item end
    local function readable(value) return not issecretvalue or not issecretvalue(value) end
    local link=item.link
    if not readable(link) or type(link)~="string" or #link==0 or #link>4096 then return item end
    local function read()
        local id=C_Item.GetItemInfoInstant(link)
        if not readable(id) or id~=item.item_id then return end
        local stats=C_Item.GetItemStats(link)
        if not readable(stats) or type(stats)~="table" then return end
        local result,count={},0
        for key,value in pairs(stats) do
            if not readable(key) or type(key)~="string" or #key>96 then return end
            if not (key:match("^ITEM_MOD_[A-Z0-9_]+_SHORT$") or key:match("^RESISTANCE[0-6]_NAME$")
                or key:match("^EMPTY_SOCKET_[A-Z_]+$")) then return end
            if not readable(value) or type(value)~="number" or value~=value
                or value < -4294967295 or value > 4294967295 then return end
            count=count+1;if count>128 then return end
            result[key]=value
        end
        return result
    end
    local ok,first=pcall(read)
    if ok and first then
        local nextOk,second=pcall(read)
        if nextOk and second then
            for key,value in pairs(first) do if second[key]~=value then return item end end
            for key in pairs(second) do if first[key]==nil then return item end end
            item.native_stats=first
        end
    end
    return item
end

local function itemEquipLocation(item)
    if AO.family~="forever" or not (C_Item and C_Item.GetItemInfoInstant) then return item end
    local function readable(value) return not issecretvalue or not issecretvalue(value) end
    local link=item.link
    if not readable(link) or type(link)~="string" or #link==0 or #link>4096 then return item end
    local function read()
        local id,_,_,location=C_Item.GetItemInfoInstant(link)
        if not readable(id) or id~=item.item_id or not readable(location) or type(location)~="string"
            or #location>64 or (location~="" and not location:match("^INVTYPE_[A-Z0-9_]+$")) then return end
        return location
    end
    local ok,first=pcall(read)
    if ok and first~=nil then
        local nextOk,second=pcall(read)
        if nextOk and first==second then item.equip_location=first end
    end
    return item
end

local function itemSellPrice(item)
    if AO.family~="forever" or not (C_Item and C_Item.GetItemInfo and C_Item.GetItemInfoInstant) then return item end
    local function readable(value) return not issecretvalue or not issecretvalue(value) end
    local link=item.link
    if not readable(link) or type(link)~="string" or #link==0 or #link>4096 or not readable(item.item_id) then return item end
    local function read()
        local id=C_Item.GetItemInfoInstant(link)
        if not readable(id) or id~=item.item_id then return end
        local _,_,_,_,_,_,_,_,_,_,price=C_Item.GetItemInfo(link)
        if readable(price) and type(price)=="number" and price%1==0 and price>=0 and price<=9999999999999 then return price end
    end
    local ok,first=pcall(read)
    if ok and first~=nil then
        local nextOk,second=pcall(read)
        if nextOk and first==second then item.sell_price_copper=first end
    end
    return item
end

local function itemRequiredLevel(item)
    item=itemEquipLocation(item)
    item=itemNativeStats(item)
    item=itemSellPrice(item)
    if AO.family~="forever" or not (C_Item and C_Item.GetItemInfo and C_Item.GetItemInfoInstant) then return item end
    local function readable(value) return not issecretvalue or not issecretvalue(value) end
    local link=item.link
    if not readable(link) or type(link)~="string" or #link==0 or #link>4096 or not readable(item.item_id) then return item end
    local ok,level=pcall(function()
        local id=C_Item.GetItemInfoInstant(link)
        if not readable(id) or id~=item.item_id then return nil end
        local _,_,_,_,required=C_Item.GetItemInfo(link)
        if readable(required) and type(required)=="number" and required%1==0
            and required>=0 and required<=4294967295 then return required end
    end)
    if ok then item.required_level=level end
    return item
end
AO.collectors.equipment = function()
    local equipment = {}
    for slot = 1, 19 do
        local id = GetInventoryItemID("player", slot)
        if id then
            equipment[tostring(slot)] = itemRequiredLevel({ item_id = id, link = GetInventoryItemLink("player", slot), guid=itemGUID(nil,slot) })
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
            slots[tostring(slot)] = itemRequiredLevel({item_id=info.itemID,count=info.stackCount,link=info.hyperlink,guid=itemGUID(bag,slot)})
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

AO.collectors.gossip_quests = function()
    local api=C_GossipInfo
    assert(AO.family=="forever" and api and GossipFrame and type(GossipFrame.IsVisible)=="function",
        "Gossip quest lists unavailable")
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted gossip observation")
        return value
    end
    local function uint(value,low,high)
        value=public(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid gossip number")
        return value
    end
    local function visible() assert(public(GossipFrame:IsVisible())==true, "Gossip panel hidden") end
    local fields={repeatable="repeatable",is_complete="isComplete",is_trivial="isTrivial",is_ignored="isIgnored"}
    local function repeatable(info)
        -- Generated API metadata and the native UI use different field names.
        local a,b=info.repeatable,info.isRepeatable
        if issecretvalue and (issecretvalue(a) or issecretvalue(b)) then return end
        if (a~=nil and type(a)~="boolean") or (b~=nil and type(b)~="boolean")
            or (a~=nil and b~=nil and a~=b) then return end
        if a~=nil then return a end
        return b
    end
    local function read(fetch,number)
        assert(type(fetch)=="function" and type(number)=="function", "Gossip list API unavailable")
        local count=uint(number(),0,128)
        local values=public(fetch())
        assert(type(values)=="table" and #values==count, "Gossip list count mismatch")
        local rows,seen={},0
        for index,info in pairs(values) do
            uint(index,1,count);info=public(info)
            assert(type(info)=="table", "Gossip quest row unavailable")
            local key=tostring(uint(info.questID,1,4294967295))
            assert(not rows[key], "Duplicate gossip quest ID")
            local row={}
            for field,native in pairs(fields) do
                local value=info[native]
                if field=="repeatable" then value=repeatable(info) end
                if (not issecretvalue or not issecretvalue(value)) and type(value)=="boolean" then row[field]=value end
            end
            rows[key]=row;seen=seen+1
        end
        assert(seen==count and uint(number(),0,128)==count, "Gossip quest list changed")
        return rows,count
    end
    visible()
    local result={coverage="visible_gossip_quest_lists"}
    for name,methods in pairs({available={api.GetAvailableQuests,api.GetNumAvailableQuests},active={api.GetActiveQuests,api.GetNumActiveQuests}}) do
        local ok,first,count=pcall(read,methods[1],methods[2])
        local again,second,laterCount=pcall(read,methods[1],methods[2])
        if ok and again and count==laterCount then
            local stable=true
            for key,row in pairs(first) do
                if not second[key] then stable=false;break end
                for field in pairs(fields) do if row[field]~=second[key][field] then row[field]=nil end end
            end
            if stable then result[name]=first end
        end
    end
    visible()
    assert(result.available or result.active, "Gossip quest lists unreadable")
    return result
end

AO.collectors.player_auras = function()
    local api=C_UnitAuras
    assert(AO.family=="forever" and api and type(api.GetUnitAuras)=="function"
        and type(api.AuraIsPrivate)=="function" and type(UnitExists)=="function"
        and type(UnitIsVisible)=="function", "Player auras unavailable")
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted aura observation")
        return value
    end
    local function uint(value)
        return (not issecretvalue or not issecretvalue(value)) and type(value)=="number"
            and value%1==0 and value>=0 and value<=4294967295
    end
    local function context()
        assert(public(UnitExists("player"))==true and public(UnitIsVisible("player"))==true,
            "Player aura context unavailable")
    end
    local function read(filter)
        local values=public(api.GetUnitAuras("player",filter,257))
        assert(type(values)=="table" and #values<=256, "Aura list unavailable or too large")
        local rows,count={},0
        for index,aura in pairs(values) do
            index=public(index);aura=public(aura)
            assert(type(index)=="number" and index%1==0 and index>=1 and index<=#values
                and type(aura)=="table", "Invalid aura list")
            local spell=public(aura.spellId)
            assert(uint(spell) and spell>0 and public(api.AuraIsPrivate(spell))==false, "Aura not public")
            local row={spell_id=spell}
            if uint(aura.applications) then row.applications=aura.applications end
            rows[tostring(index)]=row;count=count+1
        end
        assert(count==#values, "Incomplete aura list")
        return rows,count
    end
    context()
    local groups={}
    for name,filter in pairs({helpful="HELPFUL",harmful="HARMFUL"}) do
        local ok,first,count=pcall(read,filter)
        local again,second,laterCount=pcall(read,filter)
        if ok and again and count==laterCount then
            local stable=true
            for key,row in pairs(first) do
                if not second[key] or row.spell_id~=second[key].spell_id then stable=false;break end
                if row.applications~=second[key].applications then row.applications=nil end
            end
            if stable then groups[name]={entries=first} end
        end
    end
    context()
    assert(next(groups), "Player aura lists unreadable")
    return {coverage="readable_nonprivate_player_aura_lists",groups=groups}
end

AO.collectors.player_resources = function()
    assert(AO.family=="forever", "Player resources unavailable")
    local function uint(value,maximum)
        return (not issecretvalue or not issecretvalue(value)) and type(value)=="number"
            and value%1==0 and value>=0 and value<=maximum
    end
    local function health()
        assert(type(UnitHealth)=="function" and type(UnitHealthMax)=="function")
        local current,maximum=UnitHealth("player",false),UnitHealthMax("player")
        assert(uint(current,4294967295) and uint(maximum,4294967295) and current<=maximum)
        return current,maximum
    end
    local function power()
        assert(type(UnitPowerType)=="function" and type(UnitPower)=="function" and type(UnitPowerMax)=="function")
        local kind=UnitPowerType("player")
        assert(uint(kind,63))
        local current,maximum=UnitPower("player",kind,true),UnitPowerMax("player",kind,true)
        assert(uint(current,4294967295) and uint(maximum,4294967295) and current<=maximum)
        local after=UnitPowerType("player")
        assert(uint(after,63) and after==kind)
        return current,maximum,kind
    end
    local result={coverage="readable_player_resources"}
    local ok,a,b=pcall(health)
    local again,c,d=pcall(health)
    if ok and again and a==c and b==d then result.health={current=a,maximum=b} end
    local first,current,maximum,kind=pcall(power)
    local second,later,laterMax,laterKind=pcall(power)
    if first and second and current==later and maximum==laterMax and kind==laterKind then
        result.primary_power={current=current,maximum=maximum,power_type=kind,units="unmodified"}
    end
    assert(result.health or result.primary_power, "Player resources unreadable")
    return result
end

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
    if AO.family == "forever" then
        local function accessible(value) return not issecretvalue or not issecretvalue(value) end
        local bound,location=pcall(GetBindLocation)
        if bound and accessible(location) and type(location)=="string" and #location>0 and #location<=256 then
            stats.hearthstone_bind_name=location
        end
        local observed,mail=pcall(HasNewMail)
        if observed and accessible(mail) and (mail==nil or type(mail)=="boolean" or mail==1) then
            stats.new_mail_indicator=mail==true or mail==1
        end
        local function positiveID(value)
            return accessible(value) and type(value)=="number" and value%1==0 and value>=1 and value<=4294967295
        end
        local foundHearthstone,item=pcall(C_Container and C_Container.PlayerHasHearthstone)
        if foundHearthstone and positiveID(item) then
            stats.hearthstone_item_id=item
            local foundCooldown,start,duration,enabled=pcall(C_Container.GetItemCooldown,item)
            if foundCooldown and accessible(start) and accessible(duration) and accessible(enabled)
                and enabled==1 and type(start)=="number" and start>=0 and start<math.huge
                and type(duration)=="number" and duration>=0 and duration<=4294967295 then
                if start==0 and duration==0 then
                    stats.hearthstone_cooldown_seconds=0
                elseif start>0 and duration>0 then
                    local clockOk,now=pcall(GetTime)
                    if clockOk and accessible(now) and type(now)=="number" and now>=start and now<math.huge then
                        stats.hearthstone_cooldown_seconds=math.ceil(math.max(0,duration-(now-start)))
                    end
                end
            end
        end
        local ok,map=pcall(C_Map and C_Map.GetBestMapForUnit,"player")
        if ok and positiveID(map) then
            stats.map_id=map
            local found,info=pcall(C_Map.GetMapInfo,map)
            if found and accessible(info) and type(info)=="table" and positiveID(info.mapID) and info.mapID==map then
                local name,parent=info.name,info.parentMapID
                if accessible(name) and type(name)=="string" and #name>0 and #name<=256 then stats.map_name=name end
                if positiveID(parent) and parent~=map then stats.map_parent_id=parent end
            end
            local foundLevels,minLevel,maxLevel=pcall(C_Map.GetMapLevels,map)
            if foundLevels and positiveID(minLevel) and positiveID(maxLevel) and minLevel<=maxLevel and maxLevel<=255 then
                stats.map_min_level=minLevel
                stats.map_max_level=maxLevel
            end
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
    assert((AO.family=="retail" or AO.family=="forever") and GetActionInfo and ActionButtonUtil
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
    local function hotkeys(button,name,slot)
        local command=readable(button.bindingAction)
        if type(command)~="string" or #command>128 or not command:match("^[A-Z0-9_]+$")
            or #name>132 or not name:match("^[A-Za-z0-9_]+$") then return end
        local first,second=GetBindingKey(command)
        if readable(first)==nil then first,second=GetBindingKey("CLICK "..name..":LeftButton") end
        local function key(value)
            return value==nil or (type(value)=="string" and #value>0 and #value<=128)
        end
        if key(readable(first)) and key(readable(second)) then
            return {button=name,slot=slot,key1=first,key2=second}
        end
    end
    if AO.family=="forever" and type(GetBindingKey)=="function" then result.button_hotkeys={} end
    for _,prefix in ipairs(names) do
        assert(type(prefix)=="string" and #prefix<=128, "Invalid button prefix")
        for i=1,NUM_ACTIONBAR_BUTTONS do
            local name=prefix..i
            local button=_G[name]
            local slot=button and readable(button.action)
            if slot==nil then result.missing_buttons=result.missing_buttons+1
            else
                assert(type(slot)=="number" and slot%1==0 and slot>=1 and slot<=4096, "Invalid action slot")
                result.observed_buttons=result.observed_buttons+1
                if result.button_hotkeys then
                    local ok,keys=pcall(hotkeys,button,name,slot)
                    if ok then result.button_hotkeys[tostring(result.observed_buttons)]=keys end
                end
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
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted profession observation")
        return value
    end
    local function integer(value, low)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=4294967295,
            "Invalid profession number")
        return value
    end
    local function text(value)
        value=readable(value)
        assert(type(value)=="string" and #value<=16384, "Invalid profession name")
        local offset=1
        while offset<=#value do
            local first=value:byte(offset)
            local length,low,high=1,128,191
            if first<=127 then
                length=1
            elseif first>=194 and first<=223 then
                length=2
            elseif first>=224 and first<=239 then
                length=3
                if first==224 then low=160 elseif first==237 then high=159 end
            elseif first>=240 and first<=244 then
                length=4
                if first==240 then low=144 elseif first==244 then high=143 end
            else error("Invalid profession name encoding") end
            for part=1,length-1 do
                local byte=value:byte(offset+part)
                assert(byte and byte>=(part==1 and low or 128)
                    and byte<=(part==1 and high or 191), "Invalid profession name encoding")
            end
            offset=offset+length
        end
        return value
    end
    local function read()
        local result, indices = {}, {}
        local ids = {GetProfessions()}
        for slot=1,6 do
            local index=readable(ids[slot])
            if index~=nil then
                index=integer(index,1)
                local name, _, rank, maxRank, _, _, skillLine = GetProfessionInfo(index)
                name=text(name)
                local key=tostring(slot)
                indices[key]=index
                result[key]={name=name,rank=integer(rank,0),max_rank=integer(maxRank,0),
                    skill_line=integer(skillLine,1)}
            end
        end
        return result, indices
    end
    local first, indices=read()
    local second, nextIndices=read()
    for slot=1,6 do
        local key=tostring(slot)
        assert(indices[key]==nextIndices[key], "Profession slots changed")
        local before, after=first[key],second[key]
        if before then
            assert(after and before.name==after.name and before.rank==after.rank
                and before.max_rank==after.max_rank and before.skill_line==after.skill_line,
                "Profession observation changed")
        end
    end
    return first
end

AO.collectors.pet_collection_view = function()
    local api=C_PetJournal
    assert(AO.family=="forever" and PetJournal and PetJournal.IsVisible and api
        and api.GetNumPets and api.GetPetInfoByIndex and api.GetPetInfoBySpeciesID,"Pet journal unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value),"Restricted pet collection observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high,"Invalid pet collection number")
        return value
    end
    local function visible() assert(readable(PetJournal:IsVisible())==true,"Pet journal hidden") end
    local function read()
        visible()
        local count=integer(api.GetNumPets(),0,10000)
        local rows={}
        for index=1,count do
            local _,species,owned=api.GetPetInfoByIndex(index)
            species=integer(species,1,4294967295);owned=readable(owned)
            assert(type(owned)=="boolean","Unknown pet ownership row")
            local key=tostring(species)
            local row=rows[key]
            if not row then
                local name=readable(api.GetPetInfoBySpeciesID(species))
                assert(type(name)=="string" and #name>0 and #name<=256,"Invalid pet species name")
                row={name=name,owned_rows=0,unowned_rows=0};rows[key]=row
            end
            local field=owned and "owned_rows" or "unowned_rows"
            row[field]=row[field]+1
        end
        visible()
        return {coverage="filtered_journal_species",displayed_rows=count,entries=rows}
    end
    local result=read()
    local final=read()
    assert(result.displayed_rows==final.displayed_rows,"Pet journal count changed")
    for key,row in pairs(result.entries) do
        local other=final.entries[key]
        assert(other and row.name==other.name and row.owned_rows==other.owned_rows
            and row.unowned_rows==other.unowned_rows,"Pet journal changed")
    end
    for key in pairs(final.entries) do assert(result.entries[key],"Pet journal changed") end
    return result
end
AO.collectors.mail_attachments = function()
    assert(AO.family=="forever" and MailFrame and MailFrame.IsVisible and InboxFrame and InboxFrame.IsVisible
        and C_Mail and C_Mail.IsCommandPending and GetInboxNumItems and HasInboxItem and GetInboxItem,
        "Inbox unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted inbox observation")
        return value
    end
    local function integer(value,low,high)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid inbox number")
        return value
    end
    local function idle()
        assert(readable(MailFrame:IsVisible())==true and readable(InboxFrame:IsVisible())==true
            and readable(MailFrame.inboxBeingChecked)==false and readable(C_Mail.IsCommandPending())==false,
            "Inbox is not idle")
    end
    local function read()
        idle()
        local loaded,total=GetInboxNumItems()
        loaded=integer(loaded,0,100);total=integer(total,loaded,4294967295)
        local messages={}
        for message=1,loaded do
            local attachments={}
            for slot=1,16 do
                local present=readable(HasInboxItem(message,slot))
                assert(type(present)=="boolean", "Unknown inbox slot")
                if present then
                    local _,id,_,count,_,_,currency=GetInboxItem(message,slot)
                    currency=readable(currency)
                    local row={kind="unknown"}
                    if currency~=nil then
                        assert(type(currency)=="boolean", "Invalid inbox attachment kind")
                        id=readable(id);count=readable(count)
                        if id~=nil and count~=nil then
                            row={kind=currency and "currency" or "item",count=integer(count,1,4294967295)}
                            row[currency and "currency_id" or "item_id"]=integer(id,1,4294967295)
                        end
                    end
                    attachments[tostring(slot)]=row
                end
            end
            messages[tostring(message)]=attachments
        end
        idle()
        return {coverage="loaded_inbox_attachments",loaded_messages=loaded,total_messages=total,messages=messages}
    end
    local result=read()
    local final=read()
    assert(result.loaded_messages==final.loaded_messages and result.total_messages==final.total_messages,"Inbox counts changed")
    for message,rows in pairs(result.messages) do
        local other=final.messages[message]
        for slot,row in pairs(rows) do
            local nextRow=other[slot]
            assert(nextRow and row.kind==nextRow.kind and row.item_id==nextRow.item_id
                and row.currency_id==nextRow.currency_id and row.count==nextRow.count,"Inbox attachments changed")
        end
        for slot in pairs(other) do assert(rows[slot],"Inbox attachments changed") end
    end
    return result
end
AO.collectors.profession_spells = function()
    local api=C_TradeSkillUI
    local page=ProfessionsFrame and ProfessionsFrame.CraftingPage
    assert(AO.family=="forever" and page and page.IsVisible and Professions
        and Professions.InLocalCraftingMode and Professions.IsViewingExternalCraftingList
        and Professions.GetProfessionInfo and api and api.GetProfessionSpells
        and api.IsNPCCrafting and api.IsDataSourceChanging, "Profession spells unavailable")
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted profession spell observation")
        return value
    end
    local function id(value)
        value=public(value)
        assert(type(value)=="number" and value%1==0 and value>=1 and value<=4294967295, "Invalid profession spell ID")
        return value
    end
    local function context()
        assert(public(api.IsDataSourceChanging())==false, "Changing profession source")
        local inspection=page.SchematicForm and public(page.SchematicForm.isInspection)
        assert(inspection==nil or inspection==false, "Inspection profession source")
        assert(public(page:IsVisible())==true and public(Professions.InLocalCraftingMode())==true
            and public(Professions.IsViewingExternalCraftingList())==false
            and public(api.IsNPCCrafting())==false, "Nonlocal profession source")
        local info=public(Professions.GetProfessionInfo())
        assert(type(info)=="table", "Profession context unavailable")
        return id(info.professionID)
    end
    local function read(profession)
        local values=public(api.GetProfessionSpells(profession))
        assert(type(values)=="table" and #values<=4096, "Profession spell list unavailable")
        local result,count={},0
        for index,value in pairs(values) do
            index=public(index)
            assert(type(index)=="number" and index%1==0 and index>=1 and index<=#values, "Sparse profession spell list")
            local key=tostring(id(value))
            assert(not result[key], "Duplicate profession spell ID")
            result[key]={};count=count+1
        end
        assert(count==#values, "Incomplete profession spell list")
        return result
    end
    local profession=context()
    local first=read(profession)
    local second=read(profession)
    assert(context()==profession, "Profession context changed")
    for key in pairs(first) do assert(second[key], "Profession spell list changed") end
    for key in pairs(second) do assert(first[key], "Profession spell list changed") end
    return {coverage="reported_known_profession_spells",profession_id=profession,entries=first}
end

AO.collectors.profession_recipe_view = function()
    local api=C_TradeSkillUI
    local page=ProfessionsFrame and ProfessionsFrame.CraftingPage
    assert(AO.family=="forever" and page and page.IsVisible and Professions
        and Professions.InLocalCraftingMode and Professions.IsViewingExternalCraftingList
        and Professions.GetProfessionInfo and api and api.GetFilteredRecipeIDs
        and api.GetRecipeInfo and api.IsNPCCrafting and api.IsDataSourceChanging, "Local recipe list unavailable")
    local function readable(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted recipe list observation")
        return value
    end
    local function positive(value)
        value=readable(value)
        assert(type(value)=="number" and value%1==0 and value>=1 and value<=4294967295, "Invalid recipe list ID")
        return value
    end
    local function context()
        assert(readable(api.IsDataSourceChanging())==false, "Recipe list source is changing")
        local inspecting=page.SchematicForm and readable(page.SchematicForm.isInspection)
        assert(inspecting==nil or inspecting==false, "Inspection recipe list")
        assert(readable(page:IsVisible())==true and readable(Professions.InLocalCraftingMode())==true
            and readable(Professions.IsViewingExternalCraftingList())==false
            and readable(api.IsNPCCrafting())==false, "Nonlocal recipe list")
        local info=readable(Professions.GetProfessionInfo())
        assert(type(info)=="table", "Profession unavailable")
        return positive(info.professionID)
    end
    local function ids()
        local values=readable(api.GetFilteredRecipeIDs())
        assert(type(values)=="table" and #values<=4096, "Recipe list unavailable")
        local result,seen,count={},{},0
        for index,id in pairs(values) do
            assert(type(index)=="number" and index%1==0 and index>=1 and index<=#values, "Sparse recipe list")
            id=positive(id)
            assert(not seen[id], "Duplicate recipe list ID")
            seen[id]=true;result[index]=id;count=count+1
        end
        assert(count==#values, "Incomplete recipe list")
        return result
    end
    local profession=context()
    local list=ids()
    local rows={}
    for _,id in ipairs(list) do
        local info=readable(api.GetRecipeInfo(id))
        local row={}
        if info~=nil then
            assert(type(info)=="table" and positive(info.recipeID)==id, "Recipe list identity mismatch")
            local name=readable(info.name)
            if name~=nil then
                assert(type(name)=="string" and #name>0 and #name<=256, "Invalid recipe list name")
                row.name=name
            end
            local learned=readable(info.learned)
            if learned~=nil then assert(type(learned)=="boolean", "Invalid recipe learning flag");row.learned=learned end
            local function optionalSkillUp(value,kind)
                if issecretvalue and issecretvalue(value) then return end
                if kind=="bool" then if type(value)=="boolean" then return value end
                elseif type(value)=="number" and value%1==0 and value>=0 and value<=4294967295 then return value end
            end
            local difficulty=optionalSkillUp(info.relativeDifficulty,"uint")
            local labels={[0]="optimal","medium","easy","trivial"}
            row.relative_difficulty=difficulty and labels[difficulty] or nil
            row.can_skill_up=optionalSkillUp(info.canSkillUp,"bool")
            row.skill_ups_reported=optionalSkillUp(info.numSkillUps,"uint")
            -- Only keep optional metadata that agrees in another public read.
            local ok,nextInfo=pcall(api.GetRecipeInfo,id)
            if ok and (not issecretvalue or not issecretvalue(nextInfo)) and type(nextInfo)=="table"
                and (not issecretvalue or not issecretvalue(nextInfo.recipeID)) and nextInfo.recipeID==id then
                local nextDifficulty=optionalSkillUp(nextInfo.relativeDifficulty,"uint")
                if row.relative_difficulty~=(nextDifficulty and labels[nextDifficulty] or nil) then row.relative_difficulty=nil end
                if row.can_skill_up~=optionalSkillUp(nextInfo.canSkillUp,"bool") then row.can_skill_up=nil end
                if row.skill_ups_reported~=optionalSkillUp(nextInfo.numSkillUps,"uint") then row.skill_ups_reported=nil end
            else
                row.relative_difficulty=nil;row.can_skill_up=nil;row.skill_ups_reported=nil
            end
        end
        rows[tostring(id)]=row
    end
    local final=ids()
    assert(context()==profession and #final==#list, "Recipe list context changed")
    for index,id in ipairs(list) do assert(final[index]==id, "Recipe list changed") end
    return {coverage="filtered_local_recipe_list",profession_id=profession,entries=rows}
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

local function recipeCooldown(api,id)
    local function read()
        local getter=api.GetRecipeCooldown
        if (issecretvalue and issecretvalue(getter)) or type(getter)~="function" then return end
        -- Forever's schematic UI consumes seconds, daily, charges, maximum in this order.
        local seconds,daily,charges,maximum=getter(id)
        local function public(value) return not issecretvalue or not issecretvalue(value) end
        local result={}
        if public(seconds) and type(seconds)=="number" and seconds>=0 and seconds<=4294967295 then
            result.remaining_seconds=math.ceil(seconds)
            if public(daily) and type(daily)=="boolean" then result.daily=daily end
        end
        if public(charges) and public(maximum) and type(charges)=="number" and type(maximum)=="number"
            and charges%1==0 and maximum%1==0 and charges>=0 and charges<=maximum
            and maximum>=1 and maximum<=4294967295 then
            result.charges=charges;result.max_charges=maximum
        end
        if next(result) then return result end
    end
    local ok,first=pcall(read)
    local again,second=pcall(read)
    if ok and again and first and second
        and first.remaining_seconds==second.remaining_seconds and first.daily==second.daily
        and first.charges==second.charges and first.max_charges==second.max_charges then return first end
end

AO.collectors.profession_recipe_materials = function()
    local api=C_TradeSkillUI
    assert(api and api.GetRecipeInfo and api.GetRecipeSchematic and Enum and Enum.TradeskillRecipeType,
        "Recipe material APIs unavailable")
    local before=AO.collectors.profession_spells()
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted recipe materials")
        return value
    end
    local function positive(value)
        value=public(value)
        return type(value)=="number" and value%1==0 and value>=1 and value<=4294967295
    end
    local function array(value,maximum)
        value=public(value)
        assert(type(value)=="table" and #value<=maximum, "Recipe material array unavailable")
        local count=0
        for key in pairs(value) do
            assert(positive(key) and key<=#value, "Sparse recipe material array")
            count=count+1
        end
        assert(count==#value, "Incomplete recipe material array")
        return value
    end
    local function read(id)
        local info=public(api.GetRecipeInfo(id))
        assert(type(info)=="table" and public(info.recipeID)==id and public(info.learned)==true,
            "Known recipe unavailable")
        local schematic=public(api.GetRecipeSchematic(id,false))
        assert(type(schematic)=="table" and public(schematic.recipeID)==id
            and public(schematic.isRecraft)==false, "Recipe schematic unavailable")
        local kind=public(schematic.recipeType)
        assert(type(kind)=="number" and (kind==Enum.TradeskillRecipeType.Item or kind==Enum.TradeskillRecipeType.Enchant),
            "Unsupported recipe type")
        local row={status="fixed_items",reagents={}}
        for _,slot in ipairs(array(schematic.reagentSlotSchematics,64)) do
            slot=public(slot)
            assert(type(slot)=="table" and type(public(slot.required))=="boolean", "Recipe slot unavailable")
            if slot.required then
                local choices=array(slot.reagents,64)
                local variable=array(slot.variableQuantities,64)
                if #choices~=1 or #variable~=0 then return {status="unsupported_required_slots"} end
                local item=public(choices[1])
                assert(type(item)=="table", "Recipe reagent unavailable")
                local itemID,currency,quantity=public(item.itemID),public(item.currencyID),public(slot.quantityRequired)
                if currency~=nil then return {status="unsupported_required_slots"} end
                assert(positive(itemID) and positive(quantity), "Invalid recipe reagent")
                local key=tostring(itemID)
                local total=(row.reagents[key] or 0)+quantity
                assert(positive(total), "Recipe material quantity exceeds limit")
                row.reagents[key]=total
            end
        end
        local output=public(schematic.outputItemID)
        if output~=nil then
            assert(positive(output), "Invalid recipe output")
            row.output_item_id=output
        end
        if kind==Enum.TradeskillRecipeType.Item and output then
            local minimum,maximum=public(schematic.quantityMin),public(schematic.quantityMax)
            if minimum~=nil or maximum~=nil then
                assert(positive(minimum) and positive(maximum) and minimum<=maximum, "Invalid recipe output range")
                row.base_output_quantity={minimum=minimum,maximum=maximum}
            end
        end
        return row
    end
    local function equal(a,b)
        if type(a)~=type(b) then return false end
        if type(a)~="table" then return a==b end
        for key,value in pairs(a) do if not equal(value,b[key]) then return false end end
        for key in pairs(b) do if a[key]==nil then return false end end
        return true
    end
    local ids={}
    for key in pairs(before.entries) do ids[#ids+1]=tonumber(key) end
    table.sort(ids)
    local entries,retained={},0
    local function reportedCount(id)
        if type(api.GetCraftableCount)~="function" then return end
        local function readCount()
            local value=api.GetCraftableCount(id)
            if issecretvalue and issecretvalue(value) then return end
            if type(value)=="number" and value%1==0 and value>=0 and value<=4294967295 then return value end
        end
        local ok,first=pcall(readCount)
        local again,second=pcall(readCount)
        if ok and again and first~=nil and first==second then return first end
    end
    for _,id in ipairs(ids) do
        local ok,first=pcall(read,id)
        local secondOk,second=pcall(read,id)
        local row={status="unavailable"}
        if ok and secondOk and equal(first,second) then
            row=first
            local count=0
            for _ in pairs(row.reagents or {}) do count=count+1 end
            if retained+count>16384 then row={status="budget_exhausted"}
            else retained=retained+count end
        end
        if row.status=="fixed_items" then
            row.reported_craftable_count=reportedCount(id)
            row.cooldown=recipeCooldown(api,id)
        end
        entries[tostring(id)]=row
    end
    local after=AO.collectors.profession_spells()
    assert(before.profession_id==after.profession_id and equal(before.entries,after.entries),
        "Recipe material context changed")
    return {coverage="reported_known_spell_schematics",profession_id=before.profession_id,entries=entries}
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
    if info.learned then result.cooldown=recipeCooldown(C_TradeSkillUI,info.recipeID) end
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

AO.collectors.instance_context = function()
    local name,kind,difficulty,_,_,_,_,map=GetInstanceInfo()
    local types={none=true,party=true,raid=true,pvp=true,arena=true,scenario=true}
    assert(questRewardAccessible(kind) and type(kind)=="string" and types[kind], "Instance type unavailable")
    if kind=="none" then return {kind=kind} end
    assert(questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256
        and questRewardInteger(difficulty,0,4294967295), "Instance details unavailable")
    local context={kind=kind,name=name,difficulty_id=difficulty}
    if questRewardInteger(map,1,4294967295) then context.instance_map_id=map end
    if kind=="party" or kind=="raid" then
        local ok,warning=pcall(function()
            local dialog=StaticPopup_FindVisible("INSTANCE_LOCK")
            if not dialog or not dialog:IsVisible() then return end
            local seconds,_,total,completed=GetInstanceLockTimeRemaining()
            if questRewardAccessible(seconds) and type(seconds)=="number" and seconds>=0 and seconds<=4294967295
                and questRewardInteger(total,1,256) and questRewardInteger(completed,0,total)
                and dialog:IsVisible() and StaticPopup_FindVisible("INSTANCE_LOCK")==dialog then
                local result={remaining_seconds=math.ceil(seconds),encounters_total=total,encounters_completed=completed}
                if completed>0 then
                    local read,encounters=pcall(function()
                        local rows,killedCount={},0
                        for index=1,total do
                            local boss,_,killed=GetInstanceLockTimeRemainingEncounter(index)
                            if not (questRewardAccessible(boss) and type(boss)=="string" and #boss>0 and #boss<=256
                                and questRewardAccessible(killed) and type(killed)=="boolean") then return end
                            rows[tostring(index)]={name=boss,killed=killed}
                            if killed then killedCount=killedCount+1 end
                        end
                        if killedCount==completed then return rows end
                    end)
                    if read then result.encounters=encounters end
                end
                if dialog:IsVisible() and StaticPopup_FindVisible("INSTANCE_LOCK")==dialog then return result end
            end
        end)
        if ok then context.lock_warning=warning end
    end
    return context
end

AO.collectors.bag_capacity = function()
    local last=NUM_TOTAL_EQUIPPED_BAG_SLOTS or NUM_BAG_SLOTS or 4
    assert(questRewardInteger(last,4,5), "Unsupported carried bag range")
    local result={}
    for bag=0,last do
        local capacity=C_Container.GetContainerNumSlots(bag)
        assert(questRewardInteger(capacity,bag==0 and 1 or 0,1024), "Invalid bag capacity")
        local entry={total_slots=capacity,free_slots=0}
        if capacity>0 then
            local free,family=C_Container.GetContainerNumFreeSlots(bag)
            assert(questRewardInteger(free,0,capacity), "Invalid free slot count")
            entry.free_slots=free
            if questRewardInteger(family,0,4294967295) then entry.family_mask=family end
        end
        result[tostring(bag)]=entry
    end
    return result
end

AO.collectors.skill_lines = function()
    local api=C_SkillInfo
    local count=api.GetNumSkillLines()
    assert(questRewardInteger(count,1,512), "Skill lines unavailable")
    local entries={}
    for index=1,count do
        local row=api.GetSkillLineInfo(index)
        assert(questRewardAccessible(row) and type(row)=="table"
            and questRewardAccessible(row.isHeader) and type(row.isHeader)=="boolean", "Invalid skill line")
        if not row.isHeader then
            local id,name,rank,maximum=row.skillID,row.name,row.rank,row.maxRank
            assert(questRewardInteger(id,1,4294967295) and questRewardAccessible(name) and type(name)=="string"
                and #name>0 and #name<=256 and questRewardInteger(maximum,0,4294967295)
                and questRewardInteger(rank,0,maximum), "Invalid skill rank")
            local key=tostring(id)
            assert(not entries[key], "Duplicate skill line")
            local entry={name=name,rank=rank,max_rank=maximum}
            if questRewardInteger(row.skillLineCategoryID,1,4294967295) then entry.category_id=row.skillLineCategoryID end
            if questRewardInteger(row.modifier,-2147483648,2147483647) then entry.modifier=row.modifier end
            if questRewardInteger(row.tempPoints,-2147483648,2147483647) then entry.temporary_points=row.tempPoints end
            entries[key]=entry
        end
    end
    assert(questRewardInteger(api.GetNumSkillLines(),count,count), "Skill lines changed")
    return entries
end

local function petCarriedFood(api,bags)
    if type(api.CanPetEatItem)~="function" or type(bags)~="table" then return end
    local quantities,unique,slots={},0,0
    for _,bag in pairs(bags) do
        for _,item in pairs(bag) do
            slots=slots+1
            if slots>1024 or not questRewardInteger(item.item_id,1,4294967295)
                or not questRewardInteger(item.count,1,4294967295) then return end
            local id=item.item_id
            if quantities[id]==nil then
                unique=unique+1
                if unique>256 then return end
                quantities[id]=0
            end
            quantities[id]=quantities[id]+item.count
        end
    end
    local foods={}
    for id,quantity in pairs(quantities) do
        local edible=api.CanPetEatItem(id)
        if not questRewardAccessible(edible) or type(edible)~="boolean" then return end
        if edible then
            if not questRewardInteger(quantity,1,4294967295) then return end
            foods[tostring(id)]=quantity
        end
    end
    return foods
end

AO.collectors.pet_care = function(bags)
    local present=UnitExists("pet")
    assert(questRewardAccessible(present) and type(present)=="boolean", "Active pet state unavailable")
    local result={present=present}
    if not present then return result end
    local api=C_PetInfo
    local function details()
        local observed={}
        local ok,_,family=pcall(UnitCreatureFamily,"pet")
        if ok and questRewardInteger(family,1,4294967295) then observed.family_id=family end
        local read,level=pcall(UnitLevel,"pet")
        if read and questRewardInteger(level,1,4294967295) then observed.level=level end
        if api then
            local read,total,used=pcall(api.GetPetTrainingPoints)
            if read and questRewardInteger(total,-2147483648,2147483647)
                and questRewardInteger(used,-2147483648,2147483647) then
                observed.training_total=total
                observed.training_used=used
            end
        end
        return observed
    end
    local first=details()
    if api then
        local ok,happiness=pcall(api.GetPetHappiness)
        if ok and questRewardInteger(happiness,0,255) then result.happiness=happiness end
        local observed,loyalty=pcall(api.GetPetLoyalty)
        if observed and questRewardAccessible(loyalty) and type(loyalty)=="string" and #loyalty>0 and #loyalty<=256 then
            result.loyalty=loyalty
        end
        local read,foods=pcall(function()
            local rows=api.GetPetFoodTypes()
            if not (questRewardAccessible(rows) and type(rows)=="table" and #rows<=16) then return end
            local names,count={},0
            for index,name in pairs(rows) do
                if not (questRewardInteger(index,1,#rows) and questRewardAccessible(name)
                    and type(name)=="string" and #name>0 and #name<=256) then return end
                names[tostring(index)]=name
                count=count+1
            end
            if count==#rows then return names end
        end)
        if read then result.food_types=foods end
        local classified,carried=pcall(petCarriedFood,api,bags)
        if classified then result.carried_food=carried end
    end
    local current=UnitExists("pet")
    assert(questRewardAccessible(current) and current==true, "Active pet unavailable")
    local second=details()
    assert(first.family_id==second.family_id and first.level==second.level,"Active pet details changed")
    assert(first.training_total==second.training_total and first.training_used==second.training_used,
        "Active pet training points changed")
    result.family_id=first.family_id
    result.level=first.level
    if first.training_total~=nil then
        result.training_points={total=first.training_total,used=first.training_used}
    end
    return result
end

local function trainerAbilityRequirements(index)
    local count=GetTrainerServiceNumAbilityReq(index)
    if not questRewardInteger(count,0,16) then return end
    local requirements={}
    for requirement=1,count do
        local name,met=GetTrainerServiceAbilityReq(index,requirement)
        if not (questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256
            and questRewardAccessible(met) and type(met)=="boolean") then return end
        requirements[tostring(requirement)]={name=name,met=met}
    end
    return requirements
end

AO.collectors.trainer_services = function()
    local frame=ClassTrainerFrame
    assert(frame and frame:IsVisible(), "Trainer panel is not open")
    local types=Enum.TrainerType
    local kind=C_Trainer.GetTrainerType()
    assert(questRewardInteger(kind,0,4294967295)
        and (kind==types.General or kind==types.Tradeskills or kind==types.Pet), "Unsupported trainer")
    local pet=kind==types.Pet
    local function primarySlotsFull()
        local first,second=GetProfessions()
        if not questRewardAccessible(first) or not questRewardAccessible(second) then return end
        if first~=nil and not questRewardInteger(first,1,4294967295) then return end
        if second~=nil and (not questRewardInteger(second,1,4294967295) or first==nil or first==second) then return end
        return second~=nil
    end
    local slotsRead,slotsFull=false,nil
    if not pet then slotsRead,slotsFull=pcall(primarySlotsFull) end
    local count=GetNumTrainerServices()
    assert(questRewardInteger(count,1,512), "Trainer services unavailable")
    local function readEntries()
        local entries={}
        for index=1,count do
            local name,status,_,level,rank=GetTrainerServiceInfo(index)
            assert(questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256
                and questRewardAccessible(status) and (status=="available" or status=="unavailable" or status=="used"), "Invalid trainer service")
            local row={name=name,status=status}
            if questRewardInteger(level,1,255) then row.required_level=level end
            if questRewardAccessible(rank) and type(rank)=="string" and #rank>0 and #rank<=256 then row.rank_label=rank end
            if status~="used" then
                local ok,cost,isProfession=pcall(GetTrainerServiceCost,index)
                if ok and questRewardInteger(cost,0,pet and 4294967295 or 9999999999999) then
                    if pet then row.price_training_points=cost else row.price_copper=cost end
                end
                if not pet and ok and questRewardAccessible(isProfession) and type(isProfession)=="boolean" then
                    row.is_profession=isProfession
                end
                local observed,skill,required,met=pcall(GetTrainerServiceSkillReq,index)
                if observed and questRewardAccessible(skill) and type(skill)=="string" and #skill>0 and #skill<=256
                    and questRewardInteger(required,0,4294967295) and questRewardAccessible(met) and type(met)=="boolean" then
                    row.skill_requirement={name=skill,required_rank=required,met=met}
                end
                local read,abilities=pcall(trainerAbilityRequirements,index)
                if read then row.ability_requirements=abilities end
            end
            entries[tostring(index)]=row
        end
        return entries
    end
    local function same(a,b)
        if type(a)~="table" then return a==b end
        if type(b)~="table" then return false end
        for key,value in pairs(a) do if not same(value,b[key]) then return false end end
        for key in pairs(b) do if a[key]==nil then return false end end
        return true
    end
    local entries=readEntries()
    assert(same(entries,readEntries()),"Trainer lesson observations changed")
    local result={kind=pet and "pet" or (kind==types.General and "general" or "tradeskills"),entries=entries}
    if not pet then
        local read,current=pcall(primarySlotsFull)
        if slotsRead and read and type(slotsFull)=="boolean" and type(current)=="boolean" and slotsFull==current then
            result.primary_profession_slots_full=slotsFull
        end
    end
    if pet then
        local levelRead,level=pcall(function()
            local exists=UnitExists("pet")
            if questRewardAccessible(exists) and exists==true then return UnitLevel("pet") end
        end)
        if levelRead and questRewardInteger(level,1,255) then result.pet_level=level end
        local ok,points=pcall(function()
            local exists=UnitExists("pet")
            if not questRewardAccessible(exists) or exists~=true then return end
            local total,used=C_PetInfo.GetPetTrainingPoints()
            if questRewardInteger(total,0,4294967295) and questRewardInteger(used,0,4294967295) then
                return math.max(0,total-used)
            end
        end)
        if ok then result.pet_training_points_available=points end
    end
    assert(frame:IsVisible() and questRewardInteger(C_Trainer.GetTrainerType(),kind,kind)
        and questRewardInteger(GetNumTrainerServices(),count,count), "Trainer services changed")
    return result
end

local function merchantCostComponents(index)
    local count=GetMerchantItemCostInfo(index)
    if not questRewardInteger(count,1,16) then return end
    local components={}
    for component=1,count do
        local _,quantity,link,currency=GetMerchantItemCostItem(index,component)
        if not (questRewardInteger(quantity,1,4294967295) and questRewardAccessible(currency)
            and questRewardAccessible(link)) then return end
        local row={quantity=quantity}
        if currency~=nil then
            if type(currency)~="string" or #currency==0 or #currency>256 then return end
            row.currency_name=currency
        else
            if type(link)~="string" or #link==0 or #link>4096 then return end
            local id=C_Item.GetItemInfoInstant(link)
            if not questRewardInteger(id,1,4294967295) then return end
            row.item_id=id
        end
        components[tostring(component)]=row
    end
    if questRewardInteger(GetMerchantItemCostInfo(index),count,count) then return components end
end

AO.collectors.merchant_buyback = function()
    local api=C_MerchantFrame
    assert(AO.family=="forever" and MerchantFrame and type(MerchantFrame.IsVisible)=="function"
        and type(GetNumBuybackItems)=="function", "Buyback unavailable")
    local function public(value)
        assert(not issecretvalue or not issecretvalue(value), "Restricted buyback observation")
        return value
    end
    local function integer(value,low,high)
        value=public(value)
        assert(type(value)=="number" and value%1==0 and value>=low and value<=high, "Invalid buyback number")
        return value
    end
    local function context()
        assert(public(MerchantFrame:IsVisible())==true and integer(MerchantFrame.selectedTab,2,2)==2,
            "Buyback tab is not open")
    end
    local function read()
        context()
        local count=integer(GetNumBuybackItems(),0,12)
        if count>0 then assert(api and type(api.GetBuybackItemID)=="function" and type(GetBuybackItemInfo)=="function", "Buyback item APIs unavailable") end
        local entries={}
        for slot=1,count do
            local id=integer(api.GetBuybackItemID(slot),1,4294967295)
            local _,_,price,quantity=GetBuybackItemInfo(slot)
            local row={item_id=id,count=integer(quantity,1,4294967295)}
            if (not issecretvalue or not issecretvalue(price)) and type(price)=="number"
                and price%1==0 and price>=0 and price<=9999999999999 then row.buyback_price_copper=price end
            entries[tostring(slot)]=row
        end
        assert(integer(GetNumBuybackItems(),0,12)==count, "Buyback count changed")
        context()
        return entries,count
    end
    local first,count=read()
    local second,laterCount=read()
    assert(count==laterCount,"Buyback list changed")
    for key,row in pairs(first) do
        assert(second[key] and row.item_id==second[key].item_id and row.count==second[key].count, "Buyback item changed")
        if row.buyback_price_copper~=second[key].buyback_price_copper then row.buyback_price_copper=nil end
    end
    return {coverage="visible_merchant_buyback",entries=first}
end

AO.collectors.merchant_stock = function()
    local frame=MerchantFrame
    assert(frame and frame:IsVisible() and questRewardInteger(frame.selectedTab,1,1), "Merchant goods tab is not open")
    local count=GetMerchantNumItems()
    assert(questRewardInteger(count,1,256), "Merchant stock unavailable")
    local entries={}
    for index=1,count do
        local info=C_MerchantFrame.GetItemInfo(index)
        assert(questRewardAccessible(info) and type(info)=="table", "Merchant offer unavailable")
        local price,bundle,available,extended,currency=info.price,info.stackCount,info.numAvailable,info.hasExtendedCost,info.currencyID
        assert(questRewardInteger(price,0,9999999999999) and questRewardInteger(bundle,1,4294967295)
            and questRewardInteger(available,-1,2147483647) and questRewardAccessible(extended)
            and type(extended)=="boolean" and questRewardAccessible(currency), "Invalid merchant offer")
        local row={price_copper=price,bundle_size=bundle,available=available,extended_cost=extended}
        if extended then
            local ok,components=pcall(merchantCostComponents,index)
            if ok then row.cost_components=components end
        end
        if currency~=nil then
            assert(questRewardInteger(currency,1,4294967295), "Invalid merchant currency")
            row.currency_id=currency
        else
            local item=GetMerchantItemID(index)
            assert(questRewardInteger(item,1,4294967295), "Merchant item unavailable")
            row.item_id=item
        end
        entries[tostring(index)]=row
    end
    assert(frame:IsVisible() and questRewardInteger(frame.selectedTab,1,1)
        and questRewardInteger(GetMerchantNumItems(),count,count), "Merchant stock changed")
    return entries
end

AO.collectors.repair_quote = function()
    local frame=MerchantFrame
    assert(frame and frame:IsVisible() and questRewardInteger(frame.selectedTab,1,1), "Merchant goods tab is not open")
    local canRepair=CanMerchantRepair()
    assert(questRewardAccessible(canRepair) and canRepair==true, "Merchant repair unavailable")
    local cost=GetRepairAllCost()
    assert(questRewardInteger(cost,0,9999999999999) and frame:IsVisible()
        and questRewardInteger(frame.selectedTab,1,1), "Repair quote unavailable")
    return {copper=cost}
end

AO.collectors.durability_alerts = function()
    local disabled=C_GameRules.IsGameRuleActive(Enum.GameRule.RepairArmorDisabled)
    assert(questRewardAccessible(disabled) and disabled==false, "Durability alerts unavailable")
    local slots={"head","shoulders","chest","waist","legs","feet","wrists","hands","main_hand","off_hand","ranged"}
    local result={}
    for index,slot in ipairs(slots) do
        local status=GetInventoryAlertStatus(index)
        assert(questRewardInteger(status,0,2), "Unknown durability alert")
        if status==1 then result[slot]="low" end
        if status==2 then result[slot]="broken" end
    end
    return result
end

AO.collectors.equipment_durability = function()
    assert(AO.family=="forever" and type(GetInventoryItemDurability)=="function", "Equipment durability unavailable")
    local rows={}
    local function read(slot)
        local current,maximum=GetInventoryItemDurability(slot)
        if questRewardInteger(current,0,4294967295) and questRewardInteger(maximum,1,4294967295)
            and current<=maximum then return current,maximum end
    end
    for slot=1,19 do
        local ok,current,maximum=pcall(read,slot)
        if ok and current~=nil then
            local nextOk,nextCurrent,nextMaximum=pcall(read,slot)
            if nextOk and current==nextCurrent and maximum==nextMaximum then
                rows[tostring(slot)]={current=current,maximum=maximum}
            end
        end
    end
    return {coverage="readable_equipped_slots_only",entries=rows}
end

AO.collectors.weapon_enchants = function()
    assert(AO.family=="forever" and C_Item and type(C_Item.GetWeaponEnchantInfo)=="function"
        and Enum and Enum.WeaponSlot, "Weapon enchants unavailable")
    local labels={[0]="none","permanent","temporary","imbue"}
    local function read(slot)
        local values=C_Item.GetWeaponEnchantInfo(slot)
        assert(questRewardAccessible(values) and type(values)=="table" and #values<=32, "Weapon enchant list unavailable")
        local rows,count={},0
        for index,info in pairs(values) do
            assert(questRewardInteger(index,1,#values) and questRewardAccessible(info) and type(info)=="table", "Invalid weapon enchant row")
            local active,kind=info.hasEnchant,info.enchantType
            assert(questRewardAccessible(active) and type(active)=="boolean" and questRewardInteger(kind,0,3), "Invalid weapon enchant observation")
            local row={has_enchant=active,enchant_type=labels[kind]}
            if active then
                if questRewardInteger(info.enchantID,1,4294967295) then row.enchant_id=info.enchantID end
                if questRewardInteger(info.charges,0,4294967295) then row.charges=info.charges end
                local remaining=info.timeLeft
                if questRewardAccessible(remaining) and type(remaining)=="number" and remaining>=0 and remaining<=4294967295 then
                    row.remaining_milliseconds=math.ceil(remaining)
                end
            end
            rows[tostring(index)]=row;count=count+1
        end
        assert(count==#values, "Incomplete weapon enchant list")
        return rows,count
    end
    local hands={}
    for _,definition in ipairs({{"main_hand","MainHand",0},{"off_hand","OffHand",1},{"ranged","Ranged",2}}) do
        local slot=Enum.WeaponSlot[definition[2]]
        if questRewardInteger(slot,definition[3],definition[3]) then
            local ok,first,count=pcall(read,slot)
            local nextOk,second,nextCount=pcall(read,slot)
            if ok and nextOk and count==nextCount then
                local stable=true
                for index=1,count do
                    local key=tostring(index);local a,b=first[key],second[key]
                    if a.has_enchant~=b.has_enchant or a.enchant_type~=b.enchant_type or a.enchant_id~=b.enchant_id then stable=false;break end
                    if a.charges~=b.charges then b.charges=nil end
                    if a.remaining_milliseconds==nil or b.remaining_milliseconds==nil
                        or b.remaining_milliseconds>a.remaining_milliseconds then b.remaining_milliseconds=nil end
                end
                if stable then hands[definition[1]]={entries=second} end
            end
        end
    end
    return {coverage="public_weapon_enchant_lists",hands=hands}
end

AO.collectors.pet_action_bar = function()
    assert(AO.family=="forever" and type(UnitExists)=="function" and type(PetHasActionBar)=="function", "Pet action bar unavailable")
    local function context()
        local present,bar=UnitExists("pet"),PetHasActionBar()
        assert(questRewardAccessible(present) and type(present)=="boolean"
            and questRewardAccessible(bar) and type(bar)=="boolean" and (present or not bar), "Pet action context unavailable")
        return present,bar
    end
    local present,bar=context()
    local entries={}
    local function read(slot)
        local name,_,token,active,allowed,enabled,id,checks,inRange=GetPetActionInfo(slot)
        if not questRewardAccessible(token) or type(token)~="boolean" then return end
        local row
        if token then
            if not questRewardAccessible(name) or type(name)~="string" or #name>64
                or not name:match("^PET_[A-Z0-9_]+$") then return end
            row={kind="command",command_token=name}
        else
            if not questRewardInteger(id,1,4294967295) then return end
            row={kind="spell",spell_id=id}
        end
        local function flag(field,value)
            if questRewardAccessible(value) and type(value)=="boolean" then row[field]=value end
        end
        flag("is_active",active);flag("autocast_allowed",allowed);flag("autocast_enabled",enabled)
        if questRewardAccessible(checks) and type(checks)=="boolean" then
            if not checks then row.range_check_performed=false
            elseif questRewardAccessible(inRange) and type(inRange)=="boolean" then
                row.range_check_performed=true;row.in_range=inRange
            end
        end
        return row
    end
    if bar then
        assert(type(GetPetActionInfo)=="function" and questRewardInteger(NUM_PET_ACTION_SLOTS,10,10), "Pet action info unavailable")
        for slot=1,10 do
            local ok,first=pcall(read,slot)
            local nextOk,second=pcall(read,slot)
            if ok and nextOk and first and second and first.kind==second.kind
                and first.spell_id==second.spell_id and first.command_token==second.command_token then
                for _,field in ipairs({"is_active","autocast_allowed","autocast_enabled"}) do
                    if first[field]~=second[field] then second[field]=nil end
                end
                if first.range_check_performed~=second.range_check_performed or first.in_range~=second.in_range then
                    second.range_check_performed=nil;second.in_range=nil
                end
                entries[tostring(slot)]=second
            end
        end
    end
    local nextPresent,nextBar=context()
    assert(nextPresent==present and nextBar==bar, "Pet action context changed")
    return {coverage="readable_pet_action_slots",pet_present=present,has_action_bar=bar,entries=entries}
end

AO.collectors.flight_map = function()
    local frame=FlightMapFrame
    assert(frame and frame:IsVisible(), "Flight map is not open")
    local map=frame:GetMapID()
    assert(questRewardInteger(map,1,4294967295), "Flight map ID unavailable")
    local rows=C_TaxiMap.GetAllTaxiNodes(map)
    assert(questRewardAccessible(rows) and type(rows)=="table" and #rows<=512, "Flight destinations unavailable")
    local states=Enum.FlightPathState
    local labels={[states.Current]="current",[states.Reachable]="reachable",[states.Unreachable]="unreachable"}
    local entries,observed={},0
    for index,row in pairs(rows) do
        assert(questRewardInteger(index,1,#rows) and questRewardAccessible(row) and type(row)=="table", "Invalid flight destination row")
        local slot,id,name,state,transition=row.slotIndex,row.nodeID,row.name,row.state,row.isMapLayerTransition
        assert(questRewardInteger(slot,1,4096) and questRewardInteger(id,1,4294967295)
            and questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256
            and questRewardInteger(state,0,4294967295) and labels[state]
            and questRewardAccessible(transition) and type(transition)=="boolean", "Invalid flight destination")
        local key=tostring(slot)
        assert(not entries[key], "Duplicate flight destination slot")
        local entry={node_id=id,name=name,state=labels[state],map_transition=transition}
        local positioned,position=pcall(function()
            local vector=row.position
            if not questRewardAccessible(vector) or vector==nil then return end
            local function unit(value)
                return questRewardAccessible(value) and type(value)=="number" and value>=0 and value<=1
            end
            local x,y=vector:GetXY()
            if not unit(x) or not unit(y) then return end
            local nextX,nextY=vector:GetXY()
            if unit(nextX) and unit(nextY) and x==nextX and y==nextY then return {x=x,y=y} end
        end)
        if positioned then entry.map_position=position end
        if state==states.Reachable and not transition then
            local ok,cost=pcall(TaxiNodeCost,slot)
            if ok and questRewardInteger(cost,0,9999999999999) then entry.fare_copper=cost end
        end
        entries[key]=entry
        observed=observed+1
    end
    assert(observed==#rows and frame:IsVisible(), "Flight map changed during capture")
    -- Preview geometry belongs to this visible panel, not a global flight graph.
    local origin,unique,ids=nil,true,{}
    for _,entry in pairs(entries) do
        if ids[entry.node_id] then unique=false end
        ids[entry.node_id]=true
        if entry.state=="current" and not entry.map_transition then
            if origin then unique=false end
            origin=entry.node_id
        end
    end
    local function route(slot,target)
        local count=GetNumRoutes(slot)
        if not questRewardInteger(count,1,64) then return end
        local path,seen,previous={}, {},origin
        path["1"]=origin;seen[origin]=true
        for segment=1,count do
            local source=TaxiGetNodeSlot(slot,segment,true)
            local destination=TaxiGetNodeSlot(slot,segment,false)
            if not questRewardInteger(source,1,4096) or not questRewardInteger(destination,1,4096) then return end
            local from,to=entries[tostring(source)],entries[tostring(destination)]
            if not from or not to or from.map_transition or to.map_transition
                or from.node_id~=previous or seen[to.node_id] then return end
            path[tostring(segment+1)]=to.node_id;seen[to.node_id]=true;previous=to.node_id
        end
        if previous==target then return path,count end
    end
    if unique and origin then
        for slot,entry in pairs(entries) do
            if entry.state=="reachable" and not entry.map_transition then
                local ok,path,count=pcall(route,tonumber(slot),entry.node_id)
                if ok and path then
                    local nextOk,nextPath,nextCount=pcall(route,tonumber(slot),entry.node_id)
                    local same=nextOk and nextPath and count==nextCount
                    if same then
                        for index=1,count+1 do
                            local key=tostring(index)
                            if path[key]~=nextPath[key] then same=false;break end
                        end
                    end
                    if same then entry.observed_route_node_ids=path end
                end
            end
        end
    end
    local current=frame:GetMapID()
    assert(frame:IsVisible() and questRewardInteger(current,1,4294967295) and current==map, "Flight map changed during capture")
    return {map_id=map,entries=entries}
end

AO.collectors.spellbook = function()
    local api=C_SpellBook
    local bank,spellType=Enum.SpellBookSpellBank.Player,Enum.SpellBookItemType.Spell
    assert(questRewardInteger(bank,0,4294967295) and questRewardInteger(spellType,0,4294967295), "Spellbook API unavailable")
    local count=api.GetNumSpellBookSkillLines()
    assert(questRewardInteger(count,0,64), "Invalid spellbook skill line count")
    -- Project only public primitive observations; timers and GCD state are not read.
    local function resourceGetter(name,id)
        local spells=C_Spell
        assert(questRewardAccessible(spells) and type(spells)=="table", "Spell resource API unavailable")
        local getter=spells[name]
        assert(questRewardAccessible(getter) and type(getter)=="function", "Spell resource getter unavailable")
        local value=getter(id)
        assert(questRewardAccessible(value), "Restricted spell resource")
        return value
    end
    local function readCharges(id)
        local value=resourceGetter("GetSpellCharges",id)
        assert(type(value)=="table", "Spell charges unavailable")
        local current,maximum=value.currentCharges,value.maxCharges
        assert(questRewardInteger(current,0,4294967295) and questRewardInteger(maximum,1,4294967295)
            and current<=maximum, "Invalid spell charges")
        return {current=current,maximum=maximum}
    end
    local function readCastCount(id)
        local value=resourceGetter("GetSpellCastCount",id)
        assert(questRewardInteger(value,0,4294967295), "Spell cast count unavailable")
        return value
    end
    local function readCooldown(id)
        local value=resourceGetter("GetSpellCooldown",id)
        assert(type(value)=="table", "Spell cooldown state unavailable")
        local enabled,active=value.isEnabled,value.isActive
        assert(questRewardAccessible(enabled) and type(enabled)=="boolean"
            and questRewardAccessible(active) and type(active)=="boolean", "Invalid spell cooldown state")
        return {enabled=enabled,active=active}
    end
    local function resources(row,slot)
        local chargesOk,charges=pcall(readCharges,row.spell_id)
        local chargesAgain,chargesNext=pcall(readCharges,row.spell_id)
        local castsOk,casts=pcall(readCastCount,row.spell_id)
        local castsAgain,castsNext=pcall(readCastCount,row.spell_id)
        local cooldownOk,cooldown=pcall(readCooldown,row.spell_id)
        local cooldownAgain,cooldownNext=pcall(readCooldown,row.spell_id)
        local identityOk,unchanged=pcall(function()
            local getter=api.GetSpellBookItemInfo
            assert(questRewardAccessible(getter) and type(getter)=="function", "Spellbook recheck unavailable")
            local item=getter(slot,bank)
            assert(questRewardAccessible(item) and type(item)=="table", "Spellbook recheck unavailable")
            local kind,action,id=item.itemType,item.actionID,item.spellID
            return questRewardInteger(kind,0,4294967295) and questRewardInteger(action,1,4294967295)
                and questRewardInteger(id,1,4294967295) and kind==spellType
                and action==row.base_spell_id and id==row.spell_id
        end)
        if not identityOk or not unchanged then return end
        if chargesOk and chargesAgain and charges.current==chargesNext.current
            and charges.maximum==chargesNext.maximum then row.charge_count=charges end
        if castsOk and castsAgain and casts==castsNext then row.reported_cast_count=casts end
        if cooldownOk and cooldownAgain and cooldown.enabled==cooldownNext.enabled
            and cooldown.active==cooldownNext.active then row.cooldown_state=cooldown end
    end
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
                    local row={base_spell_id=item.actionID,spell_id=item.spellID}
                    local name,passive,offSpec=item.name,item.isPassive,item.isOffSpec
                    if questRewardAccessible(name) and type(name)=="string" and #name>0 and #name<=256 then row.name=name end
                    if questRewardAccessible(passive) and type(passive)=="boolean" then row.passive=passive end
                    if questRewardAccessible(offSpec) and type(offSpec)=="boolean" then row.off_spec=offSpec end
                    if C_Spell then
                        local ok,first=pcall(C_Spell.GetSpellLevelLearned,row.spell_id)
                        if ok and questRewardInteger(first,0,4294967295) then
                            local read,second=pcall(C_Spell.GetSpellLevelLearned,row.spell_id)
                            if read and questRewardInteger(second,0,4294967295) and first==second then
                                row.level_learned=first
                            end
                        end
                    end
                    if C_Spell and type(C_Spell.IsSpellUsable)=="function" then
                        local function readUsability()
                            local usable,power=C_Spell.IsSpellUsable(row.spell_id)
                            if questRewardAccessible(usable) and type(usable)=="boolean"
                                and questRewardAccessible(power) and type(power)=="boolean" then
                                return {usable=usable,insufficient_power=power}
                            end
                        end
                        local ok,first=pcall(readUsability)
                        if ok and first then
                            local nextOk,second=pcall(readUsability)
                            if nextOk and second and first.usable==second.usable
                                and first.insufficient_power==second.insufficient_power then row.usability=first end
                        end
                    end
                    if AO.family=="forever" then resources(row,slot) end
                    entries[tostring(slot)]=row
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
    local restingOk,resting=pcall(IsResting)
    if restingOk and accessible(resting) and type(resting)=="boolean" then result.resting=resting end
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
