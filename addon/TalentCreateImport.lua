-- Bounded data-only current-build creation transport.
local _,AO=...
local function number(v,low,high)
    assert(type(v)=="string" and #v<=10 and (v=="0" or v:match("^[1-9][0-9]*$")),"Invalid creation number")
    local n=tonumber(v); assert(n>=low and n<=high,"Creation number out of range"); return n
end
local function unhex(v,maximum)
    assert(type(v)=="string" and #v>0 and #v<=maximum*2 and #v%2==0 and not v:find("[^0-9a-f]"),"Invalid creation text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function contents(v)
    local value=unhex(v,16384); assert(not value:find("[^A-Za-z0-9+/=]"),"Invalid talent contents"); return value
end
function AO.DecodeTalentCreate(text)
    assert(type(text)=="string" and #text<=524288,"Creation import too large")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f>=19 and f[1]=="AO-TALCREATE-1" and f[4]=="retail" and f[10]=="create_talent_config" and f[11]=="-","Invalid creation format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"),"Invalid creation identity") end
    local count=number(f[18],0,127); assert(#f==19+5*count,"Truncated or extra creation data")
    assert(f[19]=="0" or f[19]=="1","Invalid new shared-bars expectation")
    local expected={spec_id=number(f[13],1,4294967295),active_config_id=number(f[14],1,4294967295),serialization_version=number(f[16],0,4294967295),active_import_string=contents(f[17]),entries={}}
    local selected=number(f[15],0,4294967295); if selected>0 then expected.selected_config_id=selected end
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5],1024),build=unhex(f[6],1024),interface=number(f[7],1,4294967295),character=unhex(f[8],1024),realm=unhex(f[9],1024),kind=f[10],name=unhex(f[12],64),expected=expected,expected_new_shared_action_bars=f[19]=="1"}
    assert(not op.name:find("[%c|]") and not op.name:match("^%s") and not op.name:match("%s$"),"Invalid creation name")
    for i=1,count do
        local offset=20+(i-1)*5
        local id=number(f[offset],1,4294967295); local key=tostring(id)
        assert(id~=expected.active_config_id and not expected.entries[key],"Ambiguous creation catalogue")
        assert(f[offset+2]=="0" or f[offset+2]=="1","Invalid shared-bars flag")
        local version=number(f[offset+3],0,4294967295); assert(version==expected.serialization_version,"Mixed talent versions")
        local name=unhex(f[offset+1],1024); assert(name:lower()~=op.name:lower(),"Loadout name collision")
        expected.entries[key]={name=name,uses_shared_action_bars=f[offset+2]=="1",serialization_version=version,import_string=contents(f[offset+4])}
    end
    return op
end
