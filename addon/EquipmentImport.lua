local _, AO = ...
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"), "Invalid encoded equipment field")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function number(value,low,high)
    assert(value:match("^%d+$") and #value<=10 and (value=="0" or value:sub(1,1)~="0"), "Invalid equipment integer")
    local n=tonumber(value); assert(n>=low and n<=high,"Equipment integer out of range"); return n
end
function AO.DecodeEquipmentOperation(text)
    assert(type(text)=="string" and #text<=8192,"Invalid equipment data size")
    local f={}
    for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f==15 and f[1]=="AO-EQUIP-1" and f[4]=="retail","Invalid equipment format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"),"Invalid operation identity") end
    local source,target=unhex(f[14]),unhex(f[15])
    assert(#source>0 and #source<=128 and #target<=128 and source~=target,"Invalid item identities")
    return {id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),interface=number(f[7],1,4294967295),character=unhex(f[8]),realm=unhex(f[9]),kind="equip_bag_item",bag=number(f[10],0,64),slot=number(f[11],1,1024),target_slot=number(f[12],1,19),item_id=number(f[13],1,4294967295),source_guid=source,target_guid=target}
end
