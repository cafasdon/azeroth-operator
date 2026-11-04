-- Data-only bounded transport; no native calls or evaluation.
local _,AO=...
local function number(v,low,high)
    assert(type(v)=="string" and #v<=10 and (v=="0" or v:match("^[1-9][0-9]*$")), "Invalid talent metadata number")
    local n=tonumber(v); assert(n>=low and n<=high, "Talent metadata number out of range"); return n
end
local function unhex(v,maximum,empty)
    if v=="-" then assert(empty,"Missing talent metadata text"); return "" end
    assert(type(v)=="string" and #v>0 and #v<=maximum*2 and #v%2==0 and not v:find("[^0-9a-f]"), "Invalid talent metadata text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function contents(v)
    local s=unhex(v,16384,false)
    assert(not s:find("[^A-Za-z0-9+/=]"), "Invalid talent contents")
    return s
end
function AO.DecodeTalentMetadata(text)
    assert(type(text)=="string" and #text<=524288, "Talent metadata import too large")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f>=23 and f[1]=="AO-TALMETA-1" and f[4]=="retail", "Invalid talent metadata format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"), "Invalid talent metadata identity") end
    assert(f[10]=="rename_talent_config" or f[10]=="delete_talent_config", "Invalid talent metadata kind")
    local count=number(f[18],1,128)
    assert(#f==18+5*count, "Truncated or extra talent metadata")
    local expected={spec_id=number(f[13],1,4294967295),active_config_id=number(f[14],1,4294967295),serialization_version=number(f[16],0,4294967295),active_import_string=contents(f[17]),entries={}}
    local selected=number(f[15],0,4294967295)
    if selected>0 then expected.selected_config_id=selected end
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5],1024,false),build=unhex(f[6],1024,false),interface=number(f[7],1,4294967295),character=unhex(f[8],1024,false),realm=unhex(f[9],1024,false),kind=f[10],target_config_id=number(f[11],1,4294967295),expected=expected}
    for i=1,count do
        local offset=19+(i-1)*5
        local id=number(f[offset],1,4294967295); local key=tostring(id)
        assert(id~=expected.active_config_id and not expected.entries[key], "Ambiguous talent catalogue")
        assert(f[offset+2]=="0" or f[offset+2]=="1", "Invalid shared action-bar flag")
        local version=number(f[offset+3],0,4294967295)
        assert(version==expected.serialization_version, "Mixed talent serialization versions")
        expected.entries[key]={name=unhex(f[offset+1],1024,false),uses_shared_action_bars=f[offset+2]=="1",serialization_version=version,import_string=contents(f[offset+4])}
    end
    assert(expected.entries[tostring(op.target_config_id)], "Missing target loadout")
    if op.kind=="rename_talent_config" then
        op.name=unhex(f[12],64,false)
        assert(not op.name:find("[%c|]") and not op.name:match("^%s") and not op.name:match("%s$"), "Invalid loadout name")
        for key,entry in pairs(expected.entries) do
            assert(key==tostring(op.target_config_id) or entry.name:lower()~=op.name:lower(), "Loadout name collision")
        end
    else
        assert(f[12]=="-", "Invalid loadout deletion")
    end
    return op
end
