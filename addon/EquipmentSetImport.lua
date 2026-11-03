-- Fixed-field data decoder only; never evaluates imported text.
local _, AO = ...
local function unhex(v)
    if v=="-" then return "" end
    assert(#v>0 and #v%2==0 and not v:find("[^0-9a-f]"), "Invalid set text")
    return (v:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function number(v,lo,hi,optional)
    if optional and v=="-" then return nil end
    assert(#v<=11 and (v=="0" or v:match("^[1-9][0-9]*$") or v:match("^%-[1-9][0-9]*$")), "Invalid set number")
    local n=tonumber(v); assert(n>=lo and n<=hi, "Set number out of range"); return n
end
local function boolean(v,optional)
    if optional and v=="-" then return nil end
    assert(v=="0" or v=="1", "Invalid set boolean"); return v=="1"
end
function AO.DecodeEquipmentSetOperation(text)
    assert(type(text)=="string" and #text<=8192, "Invalid set data size")
    local f={}; for field in (text..":"):gmatch("([^:]*):") do f[#f+1]=field end
    assert(#f==193 and f[1]=="AO-SET-1" and f[4]=="retail", "Invalid set format")
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find("[^0-9a-f]"), "Invalid set identity") end
    assert(f[10]=="modify_equipment_set" or f[10]=="delete_equipment_set", "Invalid set operation")
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=unhex(f[5]),build=unhex(f[6]),
        interface=number(f[7],1,4294967295),character=unhex(f[8]),realm=unhex(f[9]),
        kind=f[10],set_id=number(f[11],0,4294967295)}
    if op.kind=="modify_equipment_set" then
        op.name=unhex(f[12]); op.icon=number(f[13],1,4294967295)
        assert(#op.name>0 and #op.name<=64 and not op.name:find("[%z\1-\31\127|]")
            and not op.name:match("^%s") and not op.name:match("%s$"), "Invalid set name")
    else assert(f[12]=="-" and f[13]=="-", "Unexpected replacement metadata") end
    local expected={name=unhex(f[14]),icon=number(f[15],0,4294967295),is_equipped=boolean(f[16]),slots={}}
    assert(#expected.name>0 and #expected.name<=1024, "Invalid expected name")
    for i,key in ipairs({"num_items","num_equipped","num_in_inventory","num_lost","num_ignored"}) do
        expected[key]=number(f[16+i],0,19)
    end
    expected.assigned_spec=number(f[22],1,64,true)
    for slot=1,19 do
        local i=23+(slot-1)*9
        local s={ignored=boolean(f[i],true),raw_item_id=number(f[i+1],-2147483648,4294967295,true),
            raw_location=number(f[i+2],-2147483648,4294967295,true)}
        if f[i+3]=="-" then
            for j=i+4,i+8 do assert(f[j]=="-", "Unexpected item observation") end
        else
            local kind=unhex(f[i+3])
            assert(kind=="equipment" or kind=="bag", "Invalid observed location")
            local observed={kind=kind,slot=number(f[i+4],1,kind=="equipment" and 19 or 1024),
                item_id=number(f[i+6],1,4294967295),guid=unhex(f[i+7]),locked=boolean(f[i+8])}
            if kind=="bag" then observed.bag=number(f[i+5],0,64)
            else assert(f[i+5]=="-", "Equipment cannot contain bag") end
            assert(#observed.guid>0 and #observed.guid<=128 and not observed.guid:find("[%z\1-\31\127]"), "Invalid item identity")
            assert(s.ignored~=true and s.raw_item_id==observed.item_id and s.raw_location and s.raw_location>=0,
                "Unbound item observation")
            s.observed_item=observed
        end
        expected.slots[tostring(slot)]=s
    end
    op.expected=expected
    return op
end
