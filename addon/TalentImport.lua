local _, AO = ...
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"),"Invalid talent text")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function number(value,minimum)
    assert(value:match("^%d+$") and #value<=10 and (value=="0" or value:sub(1,1)~="0"),"Invalid talent integer")
    local n=tonumber(value); assert(n>=minimum and n<=4294967295,"Talent integer out of range"); return n
end
local function build(value)
    local text=unhex(value)
    assert(#text>0 and not text:find("[^A-Za-z0-9+/=]"),"Invalid talent build string")
    return text
end
function AO.DecodeTalentOperation(text)
    assert(type(text)=="string" and #text<=8192,"Invalid talent operation size")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f==18 and f[1]=="AO-TALENT-1" and f[4]=="retail","Invalid talent operation")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"),"Invalid operation identity") end
    assert(f[17]=="0" or f[17]=="1","Invalid shared action-bar flag")
    return {kind="load_talent_config",id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),interface=number(f[7],1),character=unhex(f[8]),realm=unhex(f[9]),spec_id=number(f[10],1),expected_active_config_id=number(f[11],1),target_config_id=number(f[12],1),serialization_version=number(f[13],0),expected_import_string=build(f[14]),desired_import_string=build(f[15]),desired_name=unhex(f[16]),uses_shared_action_bars=f[17]=="1",expected_selected_config_id=number(f[18],0)}
end
