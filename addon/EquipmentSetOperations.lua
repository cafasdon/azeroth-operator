-- Source-only metadata executor. No import/UI registration or automatic calls.
local _, AO = ...
local function readable(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted equipment set state")
    return v
end
local function same(a,b,depth)
    a,b=readable(a),readable(b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    assert((depth or 0)<8, "Equipment set data too deep")
    for key,value in pairs(a) do if not same(value,b[key],(depth or 0)+1) then return false end end
    for key in pairs(b) do if a[key]==nil then return false end end
    return true
end
local function sameClient(op)
    local _,build,_,interface=GetBuildInfo()
    return AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and op.build==tostring(build) and op.interface==interface and interface==AO.interface
        and op.character==UnitName("player") and op.realm==GetRealmName()
end
local function catalogue()
    assert(AO.collectors and AO.collectors.equipment_sets, "Equipment set collector unavailable")
    local sets=AO.collectors.equipment_sets()
    assert(sets.coverage=="native_catalogue_raw_slots" and sets.can_use==true
        and type(sets.entries)=="table", "Equipment sets unavailable")
    return sets.entries
end
local function validate(op)
    assert(type(op)=="table" and (op.kind=="modify_equipment_set" or op.kind=="delete_equipment_set")
        and sameClient(op), "Equipment set client mismatch")
    assert(type(op.set_id)=="number" and op.set_id%1==0 and op.set_id>=0 and op.set_id<=4294967295,
        "Invalid equipment set ID")
    assert(type(op.expected)=="table" and type(op.expected.name)=="string"
        and type(op.expected.slots)=="table", "Equipment set preimage required")
    for slot=1,19 do assert(type(op.expected.slots[tostring(slot)])=="table", "Incomplete equipment set preimage") end
    if op.kind=="modify_equipment_set" then
        -- Transport safety bounds, not a claim about the client's accepted name length.
        assert(type(op.name)=="string" and #op.name>0 and #op.name<=64
            and not op.name:find("[%z\1-\31\127|]") and op.name:match("^%s")==nil
            and op.name:match("%s$")==nil, "Invalid equipment set name")
        assert(type(op.icon)=="number" and op.icon%1==0 and op.icon>0 and op.icon<=4294967295,
            "Invalid equipment set icon")
    end
end
function AO.VerifyEquipmentSetOperation(op)
    validate(op)
    if GetCursorInfo()~=nil then return false end
    local current=catalogue()[tostring(op.set_id)]
    if op.kind=="delete_equipment_set" then return current==nil end
    if not current then return false end
    local desired={}
    for key,value in pairs(op.expected) do desired[key]=value end
    desired.name=op.name; desired.icon=op.icon
    return same(current,desired)
end
function AO.ApplyEquipmentSetOperation(op)
    validate(op)
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"),
        "Equipment set changes unavailable")
    assert(GetCursorInfo()==nil, "Cursor already occupied")
    local api=C_EquipmentSet
    local fn
    if api then
        if op.kind=="modify_equipment_set" then fn=api.ModifyEquipmentSet
        else fn=api.DeleteEquipmentSet end
    end
    assert(type(fn)=="function", "Equipment set write API unavailable")
    local entries=catalogue()
    assert(same(entries[tostring(op.set_id)],op.expected), "Equipment set contents changed")
    if op.kind=="modify_equipment_set" then
        assert(type(api.GetEquipmentSetID)=="function", "Equipment set name lookup unavailable")
        local collision=readable(api.GetEquipmentSetID(op.name))
        assert(collision==nil or collision==op.set_id, "Equipment set name already exists")
        for id,entry in pairs(entries) do
            assert(id==tostring(op.set_id) or entry.name~=op.name, "Equipment set name already exists")
        end
        if op.expected.name==op.name and op.expected.icon==op.icon then
            return {kind=op.kind,status="already_selected"}
        end
        -- Caller must persist an uncertainty checkpoint before entering this executor.
        -- One ordinary native call; never Save/Use/AssignSpec or a retry/rollback.
        fn(op.set_id,op.name,tostring(op.icon))
    else
        fn(op.set_id)
    end
    local ok,verified=pcall(AO.VerifyEquipmentSetOperation,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
