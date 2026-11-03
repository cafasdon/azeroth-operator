-- Bounded fixed-record association data. No evaluation or native calls.
local _,AO=...
local function number(v,low,high)
    assert(type(v)=="string" and #v<=10 and (v=="0" or v:match("^[1-9][0-9]*$")), "Invalid association number")
    local n=tonumber(v); assert(n>=low and n<=high, "Association number out of range"); return n
end
local function unhex(v)
    if v=="-" then return "" end
    assert(type(v)=="string" and #v>0 and #v%2==0 and not v:find("[^0-9a-f]"), "Invalid association text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
function AO.DecodeEquipmentSetSpec(text)
    assert(type(text)=="string" and #text<=524288, "Association import too large")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f>=197 and f[1]=="AO-SETSPEC-1" and f[4]=="retail", "Invalid association format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"), "Invalid association identity") end
    assert(f[10]=="assign_equipment_set_spec" or f[10]=="unassign_equipment_set_spec", "Invalid association operation")
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),
        interface=number(f[7],1,4294967295),character=unhex(f[8]),realm=unhex(f[9]),kind=f[10],
        set_id=number(f[11],0,4294967295),expected_specializations={},expected_catalogue={}}
    local specCount=number(f[14],1,64)
    local setCount=number(f[15+specCount],1,128)
    assert(#f==15+specCount+181*setCount, "Truncated or extra association data")
    local seen={}
    for i=1,specCount do
        local id=number(f[14+i],1,4294967295)
        assert(not seen[id], "Duplicate specialisation ID"); seen[id]=true
        op.expected_specializations[tostring(i)]=id
    end
    if op.kind=="assign_equipment_set_spec" then
        op.spec_index=number(f[12],1,specCount); op.spec_id=number(f[13],1,4294967295)
        assert(op.expected_specializations[tostring(op.spec_index)]==op.spec_id, "Specialisation identity mismatch")
    else assert(f[12]=="-" and f[13]=="-", "Unexpected specialisation target") end
    local owners={}
    for i=1,setCount do
        local offset=16+specCount+(i-1)*181
        local id=number(f[offset],0,4294967295); local key=tostring(id)
        assert(not op.expected_catalogue[key], "Duplicate equipment set")
        local entry=AO.DecodeEquipmentSetPreimage(f,offset+1)
        if entry.assigned_spec then
            assert(entry.assigned_spec<=specCount and owners[entry.assigned_spec]==nil, "Ambiguous association")
            owners[entry.assigned_spec]=id
        end
        op.expected_catalogue[key]=entry
    end
    assert(op.expected_catalogue[tostring(op.set_id)], "Target set absent")
    if op.spec_index and owners[op.spec_index]~=op.set_id then op.displaced_set_id=owners[op.spec_index] end
    return op
end
