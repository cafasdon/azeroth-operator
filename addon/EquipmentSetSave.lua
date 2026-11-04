-- Source-only create/save executor. Caller checkpoints uncertainty before entry.
local _, AO = ...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted equipment state")
    return v
end
local function same(a,b,depth)
    a,b=read(a),read(b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    assert((depth or 0)<8, "Equipment data too deep")
    for k,v in pairs(a) do if not same(v,b[k],(depth or 0)+1) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function integer(v,low,high)
    return type(read(v))=="number" and v%1==0 and v>=low and v<=high
end
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and (op.kind=="create_equipment_set" or op.kind=="save_equipment_set")
        and AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and op.build==tostring(build) and op.interface==interface and interface==AO.interface
        and op.character==UnitName("player") and op.realm==GetRealmName(), "Equipment set client mismatch")
    assert(type(op.name)=="string" and #op.name>0 and #op.name<=64
        and not op.name:find("[%z\1-\31\127|]") and not op.name:match("^%s")
        and not op.name:match("%s$"), "Invalid equipment set name")
    assert(integer(op.icon,1,4294967295), "Invalid equipment set icon")
    assert(type(op.expected_catalogue)=="table" and type(op.equipment_before)=="table"
        and type(op.ignored_slots)=="table", "Complete save preimages required")
    if op.kind=="create_equipment_set" then assert(op.set_id==nil, "Create cannot overwrite a set")
    else
        assert(integer(op.set_id,0,4294967295), "Invalid equipment set ID")
        local old=op.expected_catalogue[tostring(op.set_id)]
        assert(type(old)=="table" and old.name==op.name, "Save cannot rename a set")
    end
    local ignored,seen={},{}
    for i=1,19 do
        local key=tostring(i)
        local item=op.equipment_before[key]
        assert(type(item)=="table" and type(read(item.guid))=="string" and #item.guid<=128
            and not item.guid:find("[%z\1-\31\127]") and integer(item.item_id,0,4294967295)
            and (item.guid=="")== (item.item_id==0), "Invalid equipped identity")
        assert(item.guid=="" or not seen[item.guid], "Duplicate equipped identity")
        if item.guid~="" then seen[item.guid]=true end
        ignored[i]=read(op.ignored_slots[key])
        assert(type(ignored[i])=="boolean", "Explicit ignored slots required")
        -- Empty native set slots remain unresolved: never silently turn them into ignore.
        assert(ignored[i] or item.guid~="", "Nonignored empty save slots not yet qualified")
    end
    for key in pairs(op.ignored_slots) do
        local index=tonumber(key)
        assert(type(key)=="string" and integer(index,1,19) and tostring(index)==key, "Invalid ignored slot")
    end
    return ignored
end
local function catalogue()
    local data=AO.collectors.equipment_sets()
    assert(data.coverage=="native_catalogue_raw_slots" and read(data.can_use)==true
        and type(data.entries)=="table", "Equipment sets unavailable")
    return data.entries
end
local function equipped()
    local result={}
    for i=1,19 do
        local location=ItemLocation:CreateFromEquipmentSlot(i)
        local exists=read(C_Item.DoesItemExist(location))
        local guid,id=read(C_Item.GetItemGUID(location)),read(C_Item.GetItemID(location))
        assert(type(exists)=="boolean" and (exists and type(guid)=="string" and #guid>0
            and integer(id,1,4294967295) or not exists and guid==nil and id==nil), "Equipment identity unavailable")
        assert(read(IsInventoryItemLocked(i))==false, "Equipped item locked")
        assert(exists==read(C_Item.DoesItemExist(location)) and guid==read(C_Item.GetItemGUID(location))
            and id==read(C_Item.GetItemID(location)), "Equipment changed during observation")
        result[tostring(i)]={guid=guid or "",item_id=id or 0}
    end
    return result
end
local function idle()
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player")
        and GetCursorInfo()==nil, "Equipment set changes unavailable")
    for _,frame in pairs({pane=PaperDollFrame and PaperDollFrame.EquipmentManagerPane,popup=GearManagerPopupFrame}) do
        assert(type(frame.IsShown)=="function" and read(frame:IsShown())==false, "Close the equipment set editor first")
    end
    assert(type(StaticPopup_Visible)=="function", "Equipment set dialog state unavailable")
    for _,dialog in ipairs({"CONFIRM_SAVE_EQUIPMENT_SET","CONFIRM_OVERWRITE_EQUIPMENT_SET","CONFIRM_DELETE_EQUIPMENT_SET"}) do
        assert(not read(StaticPopup_Visible(dialog)), "Close the equipment set confirmation first")
    end
end
local function preconditions(op)
    idle()
    local entries=catalogue()
    assert(same(entries,op.expected_catalogue), "Equipment set catalogue changed")
    assert(same(equipped(),op.equipment_before), "Equipped items changed")
    local found=read(C_EquipmentSet.GetEquipmentSetID(op.name))
    if op.kind=="create_equipment_set" then
        assert(found==nil, "Equipment set name already exists")
        local count=0
        for _,entry in pairs(entries) do
            count=count+1; assert(entry.name~=op.name, "Equipment set name already exists")
        end
        assert(integer(MAX_EQUIPMENT_SETS_PER_PLAYER,1,128) and count<MAX_EQUIPMENT_SETS_PER_PLAYER,
            "Equipment set capacity unavailable or full")
    else
        assert(found==op.set_id, "Equipment set name lookup changed")
        for id,entry in pairs(entries) do
            assert(id==tostring(op.set_id) or entry.name~=op.name, "Ambiguous equipment set name")
        end
    end
end
function AO.VerifyEquipmentSetSave(op)
    validate(op)
    if GetCursorInfo()~=nil or not same(equipped(),op.equipment_before) then return false end
    local entries=catalogue()
    local id=op.set_id
    if op.kind=="create_equipment_set" then
        id=read(C_EquipmentSet.GetEquipmentSetID(op.name))
        if not integer(id,0,4294967295) or op.expected_catalogue[tostring(id)]~=nil then return false end
    end
    local key=tostring(id)
    for oldID,old in pairs(op.expected_catalogue) do
        if oldID~=key and not same(entries[oldID],old) then return false end
    end
    for newID in pairs(entries) do
        if newID~=key and op.expected_catalogue[newID]==nil then return false end
    end
    local target=entries[key]
    if not target or target.name~=op.name or target.icon~=op.icon then return false end
    local old=op.expected_catalogue[key]
    if not same(target.assigned_spec,old and old.assigned_spec) then return false end
    local ignoredCount,itemCount=0,0
    for i=1,19 do
        local slot=tostring(i)
        local expected,actual=op.equipment_before[slot],target.slots[slot]
        if not actual or actual.ignored~=op.ignored_slots[slot] then return false end
        if op.ignored_slots[slot] then ignoredCount=ignoredCount+1
        else
            itemCount=itemCount+1
            local observed=actual.observed_item
            if actual.raw_item_id~=expected.item_id or not observed or observed.guid~=expected.guid
                or observed.item_id~=expected.item_id or observed.locked~=false
                or observed.kind~="equipment" or observed.slot~=i then return false end
        end
    end
    return target.num_ignored==ignoredCount and target.num_items==itemCount
        and target.num_equipped==itemCount and target.num_in_inventory==0 and target.num_lost==0
end
function AO.ApplyEquipmentSetSave(op)
    local ignored=validate(op)
    local api=C_EquipmentSet
    local fn
    if api then
        if op.kind=="create_equipment_set" then fn=api.CreateEquipmentSet else fn=api.SaveEquipmentSet end
    end
    assert(type(fn)=="function" and type(api.GetEquipmentSetID)=="function"
        and type(AO.WithEquipmentSetSaveSlots)=="function", "Equipment set save API unavailable")
    preconditions(op)
    if op.kind=="save_equipment_set" and AO.VerifyEquipmentSetSave(op) then
        return {kind=op.kind,status="already_selected"}
    end
    AO.WithEquipmentSetSaveSlots(ignored,function()
        -- Flag changes can synchronously invoke other addon code. Recheck before saving.
        preconditions(op)
        if op.kind=="create_equipment_set" then fn(op.name,tostring(op.icon))
        else fn(op.set_id,tostring(op.icon)) end
    end)
    local ok,verified=pcall(AO.VerifyEquipmentSetSave,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
