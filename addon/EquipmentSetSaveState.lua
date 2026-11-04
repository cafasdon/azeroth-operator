-- Internal synchronous save-state helper. Not a wire executor or an automatic job.
-- The caller must persist uncertainty and check equipment/catalogue preimages first.
local _, AO = ...
local busy = false
local function boolean(value)
    assert(not issecretvalue or not issecretvalue(value), "Restricted save-slot state")
    assert(type(value)=="boolean", "Save-slot state unavailable")
    return value
end
local function flags(api)
    local result={}
    for i=1,19 do result[i]=boolean(api.IsSlotIgnoredForSave(i)) end
    return result
end
local function equal(a,b)
    for i=1,19 do if a[i]~=b[i] then return false end end
    return true
end
function AO.WithEquipmentSetSaveSlots(desired, save)
    assert(not busy, "Equipment set save already active")
    assert(AO.family=="retail" and type(desired)=="table" and type(save)=="function",
        "Invalid equipment set save request")
    local wanted={}
    for i=1,19 do wanted[i]=boolean(desired[i]) end
    for key in pairs(desired) do
        assert(type(key)=="number" and key%1==0 and key>=1 and key<=19, "Invalid save slot")
    end
    local api=C_EquipmentSet
    assert(type(api)=="table" and type(api.IsSlotIgnoredForSave)=="function"
        and type(api.IgnoreSlotForSave)=="function" and type(api.UnignoreSlotForSave)=="function",
        "Equipment set save API unavailable")
    local original=flags(api)
    assert(equal(original,flags(api)), "Save-slot state changed during observation")
    local touched={}
    busy=true
    local ok=pcall(function()
        for i=1,19 do
            -- Synchronous native events may run other addon code between calls.
            assert(boolean(api.IsSlotIgnoredForSave(i))==original[i], "Save-slot state changed")
            if original[i]~=wanted[i] then
                touched[i]=true -- a native exception may occur after its effect
                if wanted[i] then api.IgnoreSlotForSave(i) else api.UnignoreSlotForSave(i) end
                assert(boolean(api.IsSlotIgnoredForSave(i))==wanted[i], "Save-slot update unconfirmed")
            end
        end
        assert(equal(flags(api),wanted), "Save-slot preparation changed")
        save() -- exactly once; no retry and no inference from its return value
        assert(equal(flags(api),wanted), "Save-slot state changed during save")
    end)
    local restored=true
    for i=1,19 do
        if touched[i] then
            local clean=pcall(function()
                local current=boolean(api.IsSlotIgnoredForSave(i))
                if current~=original[i] then
                    -- Boolean state has no ownership token. A same-value external edit
                    -- cannot be distinguished; callers must exclude competing save UI.
                    if original[i] then api.IgnoreSlotForSave(i) else api.UnignoreSlotForSave(i) end
                end
                assert(boolean(api.IsSlotIgnoredForSave(i))==original[i], "Save-slot restoration unconfirmed")
            end)
            if not clean then restored=false end
        end
    end
    local checked,matches=pcall(function() return equal(flags(api),original) end)
    busy=false
    assert(restored and checked and matches, "Equipment set save state not restored; inspect before continuing")
    assert(ok, "Equipment set save not confirmed; do not retry")
    return true -- only temporary state restored; caller must verify saved contents
end
