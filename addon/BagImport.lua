local _, AO = ...
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"), "Invalid encoded bag field")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function number(value,low,high)
    assert(value:match("^%d+$") and #value<=10 and (value=="0" or value:sub(1,1)~="0"), "Invalid bag integer")
    local n=tonumber(value); assert(n>=low and n<=high,"Bag integer out of range"); return n
end
function AO.DecodeBagOperation(text)
    assert(type(text)=="string" and #text<=8192,"Invalid bag data size")
    local f={}
    for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f==16 and f[1]=="AO-BAG-1" and f[4]=="retail","Invalid bag format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"),"Invalid operation identity") end
    local guid=unhex(f[15]); assert(#guid>0 and #guid<=128 and not guid:find('%c'),"Invalid item identity")
    return {id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),interface=number(f[7],1,4294967295),character=unhex(f[8]),realm=unhex(f[9]),kind="move_bag_item",bag=number(f[10],0,64),slot=number(f[11],1,1024),target_bag=number(f[12],0,64),target_slot=number(f[13],1,1024),item_id=number(f[14],1,4294967295),source_guid=guid,count=number(f[16],1,2147483647),expect_empty_target=true}
end
