-- Called by an explicit player apply action. Never runs macro bodies.
local _, AO = ...

local function limits()
    local constants = Constants and Constants.MacroConsts
    local account = (constants and constants.MAX_ACCOUNT_MACROS) or MAX_ACCOUNT_MACROS
    local character = (constants and constants.MAX_CHARACTER_MACROS) or MAX_CHARACTER_MACROS
    assert(type(account)=="number" and account>0 and account<=1024, "Macro account limit unavailable")
    assert(type(character)=="number" and character>0 and character<=1024, "Macro character limit unavailable")
    return account, character
end

local function scope(value)
    assert(value=="account" or value=="character", "Invalid macro scope")
    return value
end

local function readable(value)
    assert(not issecretvalue or not issecretvalue(value), "Restricted macro value")
    return value
end

local function info(index)
    local name, icon, body = GetMacroInfo(index)
    return {name=readable(name), icon=readable(icon), body=readable(body)}
end

local function equal(a,b)
    return a.name==b.name and a.icon==b.icon and a.body==b.body
end

local function fields(value)
    assert(type(value)=="table", "Macro fields required")
    assert(type(value.name)=="string" and #value.name>0 and #value.name<=16 and not value.name:find("[%z\1-\31]"), "Invalid macro name")
    assert(type(value.body)=="string" and #value.body<=255 and not value.body:find("%z"), "Invalid macro body")
    assert(type(value.icon)=="number" and value.icon>0 and value.icon%1==0, "Numeric macro icon required")
end

local function inventory(which)
    scope(which)
    local accountLimit, characterLimit = limits()
    local account, character = GetNumMacros()
    local count = which=="account" and account or character
    local maximum = which=="account" and accountLimit or characterLimit
    assert(type(count)=="number" and count>=0 and count<=maximum and count%1==0, "Invalid macro count")
    local base = which=="account" and 0 or accountLimit
    local result = {}
    for offset=1,count do
        local index=base+offset
        result[#result+1]={index=index, value=info(index)}
    end
    return result, count, maximum
end

local function locate(which, expected)
    local found
    for _, entry in ipairs(inventory(which)) do
        if equal(entry.value,expected) then
            assert(not found, "Ambiguous macro identity")
            found=entry.index
        end
    end
    assert(found, "Macro changed or missing")
    return found
end

local function destination(which, desired, exceptIndex)
    local entries,count,maximum=inventory(which)
    for _, entry in ipairs(entries) do
        assert(entry.index==exceptIndex or entry.value.name~=desired.name, "Destination macro name exists")
    end
    if not exceptIndex then assert(count<maximum, "Destination macro scope full") end
end

function AO.ApplyMacroOperation(operation)
    assert(not InCombatLockdown(), "Macro changes unavailable in combat")
    assert(AO.family=="retail", "Macro mutations not qualified for this client family")
    local _,_,_,interface=GetBuildInfo()
    assert(AO.interface and AO.interface==interface, "Client interface mismatch")
    assert(type(operation)=="table", "Macro operation required")
    local kind=operation.kind
    assert(kind=="create" or kind=="edit" or kind=="delete" or kind=="move", "Unsupported macro operation")
    assert(GetMacroInfo and GetNumMacros, "Macro read API unavailable")
    local source,index
    if kind~="create" then
        source=scope(operation.source_scope)
        fields(operation.expected)
        index=locate(source,operation.expected)
    end
    if kind=="create" or kind=="edit" then fields(operation.desired) end
    local target = kind=="edit" and source or operation.target_scope
    if kind=="create" or kind=="move" then scope(target) end
    if kind=="move" then assert(target~=source, "Macro is already in requested scope") end
    if kind=="create" or kind=="edit" or kind=="move" then
        destination(target,kind=="move" and operation.expected or operation.desired,kind=="edit" and index or nil)
    end
    assert((kind~="create" and kind~="move") or CreateMacro, "CreateMacro unavailable")
    assert(kind~="edit" or EditMacro, "EditMacro unavailable")
    assert((kind~="delete" and kind~="move") or DeleteMacro, "DeleteMacro unavailable")
    -- Each call below may have taken effect even when its return/readback fails.
    -- The caller must keep an outcome-unknown receipt and never auto-retry.
    if kind=="create" then
        local d=operation.desired
        CreateMacro(d.name,d.icon,d.body,target=="character")
        return {status="applied",kind=kind,index=locate(target,d),scope=target}
    elseif kind=="edit" then
        local d=operation.desired
        EditMacro(index,d.name,d.icon,d.body)
        return {status="applied",kind=kind,index=locate(source,d),scope=source}
    elseif kind=="delete" then
        DeleteMacro(index)
        for _, entry in ipairs(inventory(source)) do
            assert(not equal(entry.value,operation.expected), "Deletion readback failed")
        end
        return {status="applied",kind=kind,scope=source}
    else
        local d=operation.expected
        CreateMacro(d.name,d.icon,d.body,target=="character")
        local created=locate(target,d)
        local ok=pcall(function()
            -- CreateMacro can renumber entries: resolve source again.
            DeleteMacro(locate(source,d))
            for _, entry in ipairs(inventory(source)) do
                assert(not equal(entry.value,d), "Source deletion readback failed")
            end
        end)
        return {status=ok and "applied" or "copied_source_removal_unconfirmed",kind=kind,
            index=created,scope=target,action_bar_references="not_relocated"}
    end
end
