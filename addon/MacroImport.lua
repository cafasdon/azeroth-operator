-- Fixed data fields only. No Lua evaluation, network input or automatic execution.
local _, AO = ...
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"), "Invalid encoded text")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function integer(value)
    assert(value:match("^[1-9][0-9]*$") and #value<=10, "Invalid integer")
    local result=tonumber(value)
    assert(result<=4294967295, "Integer out of range")
    return result
end
function AO.DecodeMacroOperation(text)
    assert(type(text)=="string" and #text<=8192, "Operation too large")
    local fields={}
    for field in (text.."|"):gmatch("([^|]*)|") do fields[#fields+1]=field end
    assert(#fields==18 and fields[1]=="AO-MACRO-1", "Invalid operation format")
    for _,index in ipairs({2,3}) do
        assert(#fields[index]==64 and not fields[index]:find("[^0-9a-f]"), "Invalid operation identity")
    end
    assert(fields[4]=="retail", "Unsupported family")
    local operation={id=fields[2],snapshot_id=fields[3],family=fields[4],
        variant=unhex(fields[5]),build=unhex(fields[6]),interface=integer(fields[7]),
        character=unhex(fields[8]),realm=unhex(fields[9]),kind=fields[10]}
    assert(operation.kind=="create" or operation.kind=="edit" or operation.kind=="delete" or operation.kind=="move", "Unsupported operation")
    operation.source_scope=fields[11]~="-" and fields[11] or nil
    operation.target_scope=fields[12]~="-" and fields[12] or nil
    if operation.kind~="create" then
        operation.expected={name=unhex(fields[13]),icon=integer(fields[14]),body=unhex(fields[15])}
    else assert(fields[11]=="-" and fields[13]=="-" and fields[14]=="-" and fields[15]=="-", "Unexpected source fields") end
    if operation.kind=="create" or operation.kind=="edit" then
        operation.desired={name=unhex(fields[16]),icon=integer(fields[17]),body=unhex(fields[18])}
    else assert(fields[16]=="-" and fields[17]=="-" and fields[18]=="-", "Unexpected desired fields") end
    if operation.kind=="edit" or operation.kind=="delete" then assert(fields[12]=="-", "Unexpected target scope") end
    return operation
end

-- Keep the exact reviewed bytes private to this module. Editing the input invalidates
-- the review; an Apply event cannot silently substitute different operation data.
local reviewedText
function AO.InvalidateMacroReview()
    reviewedText=nil
end
function AO.ReviewMacroImport(text)
    reviewedText=nil
    local operation=AO.DecodeMacroOperation(text)
    reviewedText=text
    return operation
end
function AO.ApplyReviewedMacro(text)
    assert(reviewedText and text==reviewedText, "Review the current operation before applying")
    reviewedText=nil
    return AO.ApplyImportedMacro(text)
end

-- The review UI must call this only from the player's explicit Apply action.
function AO.ApplyImportedMacro(text)
    local operation=AO.DecodeMacroOperation(text)
    local _,build,_,interface=GetBuildInfo()
    assert(operation.family==AO.family and operation.variant==AO.variant and operation.build==tostring(build)
        and operation.interface==interface, "Operation targets a different client")
    assert(operation.character==UnitName("player") and operation.realm==GetRealmName(), "Operation targets a different character")
    assert(not InCombatLockdown(), "Macro changes unavailable in combat")
    AzerothOperatorOperations=AzerothOperatorOperations or {}
    local receipts=AzerothOperatorOperations
    assert(not receipts[operation.id], "Operation already attempted; reconcile before retry")
    local count=0
    for _ in pairs(receipts) do count=count+1 end
    assert(count<128, "Operation receipt limit reached")
    receipts[operation.id]={status="outcome_unknown",kind=operation.kind}
    local ok,result=pcall(AO.ApplyMacroOperation,operation)
    if ok then receipts[operation.id]=result end
    -- SavedVariables are persisted only at reload/logout; never promise crash durability.
    return receipts[operation.id],ok
end
