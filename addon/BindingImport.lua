local _, AO = ...
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"), "Invalid encoded binding field")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
function AO.DecodeBindingOperation(text)
    assert(type(text)=="string" and #text<=8192, "Invalid binding data size")
    local f={}
    for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f==14 and f[1]=="AO-BIND-1" and f[4]=="retail", "Invalid binding format")
    for _,index in ipairs({2,3}) do assert(#f[index]==64 and not f[index]:find("[^0-9a-f]"), "Invalid binding identity") end
    assert(f[7]:match("^[1-9][0-9]*$") and #f[7]<=10 and tonumber(f[7])<=4294967295, "Invalid interface")
    assert(f[13]=="1" or f[13]=="2", "Invalid binding set")
    assert(f[14]:match("^[0-9]$"), "Invalid binding context")
    return {id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),interface=tonumber(f[7]),character=unhex(f[8]),realm=unhex(f[9]),kind="set_binding",key=unhex(f[10]),expected=unhex(f[11]),desired=unhex(f[12]),binding_set=tonumber(f[13]),context=tonumber(f[14])}
end
