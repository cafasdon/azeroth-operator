local _, AO = ...
local function unhex(v)
    if v=="-" then return "" end
    assert(#v>0 and #v%2==0 and not v:find("[^0-9a-f]"),"Invalid action text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function number(v,hi)
    assert(v:match("^[1-9][0-9]*$") and #v<=10,"Invalid action number")
    local n=tonumber(v); assert(n<=hi,"Action number out of range"); return n
end
local function action(f,i)
    local kind=f[i]
    if kind=="-" then
        for j=i+1,i+5 do assert(f[j]=="-","Empty action contains data") end
        return {empty=true}
    end
    assert(kind=="spell" or kind=="item" or kind=="macro","Unsupported action kind")
    local sub=unhex(f[i+2])
    local result={empty=false,kind=kind,id=number(f[i+1],4294967295),sub_type=sub~="" and sub or nil}
    if kind=="macro" then
        result.macro={name=unhex(f[i+3]),icon=number(f[i+4],4294967295),body=unhex(f[i+5])}
    else for j=i+3,i+5 do assert(f[j]=="-","Unexpected macro contents") end end
    return result
end
function AO.DecodeActionOperation(text)
    assert(type(text)=="string" and #text<=8192,"Invalid action data size")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    local copying=f[1]=="AO-ACTIONCOPY-1"
    assert(#f==23 and (f[1]=="AO-ACTION-1" or copying) and f[4]=="retail","Invalid action format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"),"Invalid action identity") end
    local result={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),
        interface=number(f[7],4294967295),character=unhex(f[8]),realm=unhex(f[9]),kind=copying and "copy_action_slot" or "move_action_slot",
        source_slot=number(f[10],4096),target_slot=number(f[11],4096),source=action(f,12),target=action(f,18)}
    assert(not result.source.empty and result.source_slot~=result.target_slot,"Invalid action move")
    assert(not copying or result.target.empty,"Copy destination must be explicitly empty")
    return result
end
