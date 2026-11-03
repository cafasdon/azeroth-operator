-- Source-only: not packaged or dispatched. Caller must checkpoint uncertainty.
local _, AO = ...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted equipment state")
    return v
end
local function same(a,b,depth)
    a,b=read(a),read(b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    assert((depth or 0)<8, "Equipment set data too deep")
    for k,v in pairs(a) do if not same(v,b[k],(depth or 0)+1) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and op.kind=="use_equipment_set" and AO.family=="retail"
        and op.family==AO.family and op.variant==AO.variant and op.build==tostring(build)
        and op.interface==interface and interface==AO.interface
        and op.character==UnitName("player") and op.realm==GetRealmName(), "Equipment set client mismatch")
    assert(type(op.set_id)=="number" and op.set_id%1==0 and op.set_id>=0 and op.set_id<=4294967295,
        "Invalid equipment set ID")
    assert(type(op.expected)=="table" and type(op.expected.slots)=="table"
        and op.expected.num_lost==0 and type(op.equipment_before)=="table", "Complete equipment preimage required")
    local desired,seen={},{}
    for i=1,19 do
        local key=tostring(i)
        local before=read(op.equipment_before[key])
        assert(type(before)=="string" and #before<=128 and not before:find("[%z\1-\31\127]"), "Invalid equipment identity")
        local slot=op.expected.slots[key]
        assert(type(slot)=="table", "Incomplete equipment set")
        if read(slot.ignored)==true then desired[key]=before
        else
            local item=slot.observed_item
            assert(read(slot.ignored)==false and type(item)=="table" and read(item.locked)==false
                and (item.kind=="bag" or item.kind=="equipment") and type(read(slot.raw_item_id))=="number"
                and slot.raw_item_id>0 and item.item_id==slot.raw_item_id
                and type(read(item.guid))=="string" and #item.guid>0 and #item.guid<=128
                and not item.guid:find("[%z\1-\31\127]"), "Unresolved equipment set slot")
            desired[key]=item.guid
        end
        assert(desired[key]=="" or not seen[desired[key]], "Duplicate target item identity")
        if desired[key]~="" then seen[desired[key]]=true end
    end
    return desired
end
local function catalogue(op)
    local data=AO.collectors.equipment_sets()
    assert(data.coverage=="native_catalogue_raw_slots" and read(data.can_use)==true, "Equipment sets unavailable")
    return data.entries[tostring(op.set_id)]
end
local function equipped()
    local result={}
    for i=1,19 do
        local location=ItemLocation:CreateFromEquipmentSlot(i)
        local exists=read(C_Item.DoesItemExist(location))
        local guid=read(C_Item.GetItemGUID(location))
        assert(type(exists)=="boolean" and (exists and type(guid)=="string" and #guid>0
            or not exists and guid==nil), "Equipment identity unavailable")
        assert(read(IsInventoryItemLocked(i))==false, "Equipment slot locked")
        assert(exists==read(C_Item.DoesItemExist(location)) and guid==read(C_Item.GetItemGUID(location)),
            "Equipment changed during observation")
        result[tostring(i)]=guid or ""
    end
    return result
end
function AO.VerifyEquipmentSetUse(op)
    local desired=validate(op)
    if GetCursorInfo()~=nil then return false end
    local current=catalogue(op)
    if not current then return false end
    for _,key in ipairs({"name","icon","assigned_spec"}) do
        if not same(current[key],op.expected[key]) then return false end
    end
    for i=1,19 do
        local key=tostring(i)
        local actual,expected=current.slots[key],op.expected.slots[key]
        if not actual or not same(actual.ignored,expected.ignored)
            or not same(actual.raw_item_id,expected.raw_item_id) then return false end
    end
    return same(equipped(),desired)
end
function AO.ApplyEquipmentSetUse(op)
    local desired=validate(op)
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"),
        "Equipment set changes unavailable")
    assert(GetCursorInfo()==nil, "Cursor already occupied")
    assert(C_EquipmentSet and type(C_EquipmentSet.UseEquipmentSet)=="function"
        and type(C_EquipmentSet.EquipmentSetContainsLockedItems)=="function", "Equipment set API unavailable")
    assert(same(catalogue(op),op.expected), "Equipment set contents changed")
    assert(read(C_EquipmentSet.EquipmentSetContainsLockedItems(op.set_id))==false, "Equipment set items locked")
    local before=equipped()
    assert(same(before,op.equipment_before), "Equipped items changed")
    if same(before,desired) then return {kind=op.kind,status="already_selected"} end
    -- Exactly one native call. Return values/events are not completion evidence.
    C_EquipmentSet.UseEquipmentSet(op.set_id)
    local ok,verified=pcall(AO.VerifyEquipmentSetUse,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
