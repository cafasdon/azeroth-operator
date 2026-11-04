-- Bounded create/save data decoder; never evaluates text or calls native APIs.
local _,AO=...
local function number(v,low,high)
    assert(type(v)=="string" and #v<=10 and (v=="0" or v:match("^[1-9][0-9]*$")), "Invalid save number")
    local n=tonumber(v); assert(n>=low and n<=high, "Save number out of range"); return n
end
local function unhex(v)
    if v=="-" then return "" end
    assert(type(v)=="string" and #v>0 and #v%2==0 and not v:find("[^0-9a-f]"), "Invalid save text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
function AO.DecodeEquipmentSetSave(text)
    assert(type(text)=="string" and #text<=524288, "Save import too large")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f>=71 and f[1]=="AO-SETSAVE-1" and f[4]=="retail", "Invalid save format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"), "Invalid save identity") end
    assert(f[10]=="create_equipment_set" or f[10]=="save_equipment_set", "Invalid save operation")
    local count=number(f[14],0,128)
    assert(#f==71+181*count, "Truncated or extra save data")
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),
        interface=number(f[7],1,4294967295),character=unhex(f[8]),realm=unhex(f[9]),kind=f[10],
        name=unhex(f[12]),icon=number(f[13],1,4294967295),equipment_before={},ignored_slots={},expected_catalogue={}}
    assert(#op.name>0 and #op.name<=64 and not op.name:find("[%z\1-\31\127|]")
        and not op.name:match("^%s") and not op.name:match("%s$"), "Invalid save name")
    if op.kind=="create_equipment_set" then assert(f[11]=="-" and count<128, "Create cannot overwrite a set")
    else op.set_id=number(f[11],0,4294967295) end
    local seen={}
    for slot=1,19 do
        local i=15+(slot-1)*3; local key=tostring(slot)
        local guid,id=unhex(f[i]),number(f[i+1],0,4294967295)
        assert(#guid<=128 and not guid:find("[%z\1-\31\127]") and (guid=="")== (id==0), "Invalid equipped identity")
        assert(guid=="" or not seen[guid], "Duplicate equipped identity")
        if guid~="" then seen[guid]=true end
        assert(f[i+2]=="0" or f[i+2]=="1", "Invalid ignored flag")
        local ignored=f[i+2]=="1"
        assert(ignored or guid~="", "Nonignored empty save slots not yet qualified")
        op.equipment_before[key]={guid=guid,item_id=id}; op.ignored_slots[key]=ignored
    end
    for i=1,count do
        local offset=72+(i-1)*181
        local id=number(f[offset],0,4294967295); local key=tostring(id)
        assert(not op.expected_catalogue[key], "Duplicate equipment set")
        local entry=AO.DecodeEquipmentSetPreimage(f,offset+1)
        assert(id==op.set_id or entry.name:lower()~=op.name:lower(), "Equipment set name already exists")
        op.expected_catalogue[key]=entry
    end
    if op.kind=="save_equipment_set" then
        local old=op.expected_catalogue[tostring(op.set_id)]
        assert(old and old.name==op.name, "Save target missing or renamed")
    end
    return op
end
