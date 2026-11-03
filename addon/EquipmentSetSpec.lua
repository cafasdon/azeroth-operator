-- Source-only association executor. Not packaged; caller must checkpoint uncertainty.
local _, AO = ...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted equipment set state")
    return v
end
local function same(a,b,depth)
    a,b=read(a),read(b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    assert((depth or 0)<9, "Equipment set data too deep")
    for k,v in pairs(a) do if not same(v,b[k],(depth or 0)+1) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function integer(v,lo,hi)
    v=read(v); return type(v)=="number" and v%1==0 and v>=lo and v<=hi
end
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and (op.kind=="assign_equipment_set_spec" or op.kind=="unassign_equipment_set_spec")
        and AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and op.build==tostring(build) and op.interface==interface and interface==AO.interface
        and op.character==UnitName("player") and op.realm==GetRealmName(), "Equipment set client mismatch")
    assert(integer(op.set_id,0,4294967295) and type(op.expected_catalogue)=="table"
        and type(op.expected_catalogue[tostring(op.set_id)])=="table", "Equipment set catalogue required")
    local count=read(GetNumSpecializations())
    assert(integer(count,1,64), "Specialisations unavailable")
    local owners,n={},0
    for id,entry in pairs(op.expected_catalogue) do
        assert(type(id)=="string" and integer(tonumber(id),0,4294967295) and tostring(tonumber(id))==id
            and type(entry)=="table" and type(entry.name)=="string" and type(entry.slots)=="table", "Invalid catalogue entry")
        for slot=1,19 do assert(type(entry.slots[tostring(slot)])=="table", "Incomplete set slots") end
        local spec=read(entry.assigned_spec)
        if spec~=nil then
            assert(integer(spec,1,count) and owners[spec]==nil, "Ambiguous specialisation association")
            owners[spec]=tonumber(id)
        end
        n=n+1; assert(n<=128, "Equipment set catalogue too large")
    end
    if op.kind=="assign_equipment_set_spec" then
        assert(integer(op.spec_index,1,count) and integer(op.spec_id,1,4294967295), "Invalid specialisation")
        assert(C_SpecializationInfo and type(C_SpecializationInfo.GetSpecializationInfo)=="function"
            and read(C_SpecializationInfo.GetSpecializationInfo(op.spec_index))==op.spec_id, "Specialisation identity changed")
    else assert(op.spec_index==nil and op.spec_id==nil, "Unexpected specialisation target") end
    return count
end
local function catalogue(count)
    local data=AO.collectors.equipment_sets()
    assert(data.coverage=="native_catalogue_raw_slots" and read(data.can_use)==true
        and type(data.entries)=="table", "Equipment sets unavailable")
    assert(C_EquipmentSet and type(C_EquipmentSet.GetEquipmentSetForSpec)=="function", "Association lookup unavailable")
    local owners={}
    for id,entry in pairs(data.entries) do
        local spec=read(entry.assigned_spec)
        if spec~=nil then
            assert(integer(spec,1,count) and owners[spec]==nil, "Ambiguous current associations")
            owners[spec]=tonumber(id)
        end
    end
    for spec=1,count do
        assert(read(C_EquipmentSet.GetEquipmentSetForSpec(spec))==owners[spec], "Association lookup disagrees")
    end
    return data.entries
end
local function desired(op)
    local result={}
    for id,entry in pairs(op.expected_catalogue) do
        local out={}; for k,v in pairs(entry) do out[k]=v end
        if id==tostring(op.set_id) then out.assigned_spec=op.spec_index
        elseif op.spec_index and entry.assigned_spec==op.spec_index then out.assigned_spec=nil end
        result[id]=out
    end
    return result
end
function AO.VerifyEquipmentSetSpec(op)
    local count=validate(op)
    if GetCursorInfo()~=nil then return false end
    return same(catalogue(count),desired(op))
end
function AO.ApplyEquipmentSetSpec(op)
    local count=validate(op)
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player")
        and GetCursorInfo()==nil, "Equipment set changes unavailable")
    local fn
    if C_EquipmentSet then
        if op.kind=="assign_equipment_set_spec" then fn=C_EquipmentSet.AssignSpecToEquipmentSet
        else fn=C_EquipmentSet.UnassignEquipmentSetSpec end
    end
    assert(type(fn)=="function", "Association write API unavailable")
    local before=catalogue(count)
    assert(same(before,op.expected_catalogue), "Equipment set catalogue changed")
    if same(before,desired(op)) then return {kind=op.kind,status="already_selected"} end
    -- One native operation; no explicit Unassign-before-Assign, spec switch, equip or retry.
    if op.kind=="assign_equipment_set_spec" then fn(op.set_id,op.spec_index) else fn(op.set_id) end
    local ok,verified=pcall(AO.VerifyEquipmentSetSpec,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
