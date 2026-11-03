-- Source-only player Apply executor. Not registered with import/UI until qualified.
local _, AO = ...
local function readable(value)
    assert(not issecretvalue or not issecretvalue(value), "Restricted action state")
    return value
end
local function matches(slot,expected)
    local kind,id,subType=GetActionInfo(slot)
    kind,id,subType=readable(kind),readable(id),readable(subType)
    if expected.empty then return kind==nil end
    if kind~=expected.kind or id~=expected.id or subType~=expected.sub_type then return false end
    if kind=="macro" then
        local name,icon,body=GetMacroInfo(id)
        return readable(name)==expected.macro.name and readable(icon)==expected.macro.icon
            and readable(body)==expected.macro.body
    end
    return true
end
local function validateAction(action)
    assert(type(action)=="table" and type(action.empty)=="boolean", "Missing action preimage")
    if action.empty then return end
    assert(action.kind=="spell" or action.kind=="item" or action.kind=="macro", "Action kind not supported")
    assert(type(action.id)=="number" and action.id%1==0 and action.id>0, "Invalid action ID")
    if action.kind=="macro" then
        assert(type(action.macro)=="table" and type(action.macro.name)=="string"
            and type(action.macro.body)=="string" and action.macro.icon~=nil, "Macro contents required")
    end
end
local function cursorMatches(action)
    local kind,id,_,spellID=GetCursorInfo()
    kind,id,spellID=readable(kind),readable(id),readable(spellID)
    if action.empty then return kind==nil end
    if kind~=action.kind then return false end
    return (kind=="spell" and spellID or id)==action.id
end
local function sameClient(op)
    local _,build,_,interface=GetBuildInfo()
    return AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and op.build==tostring(build) and op.interface==interface and interface==AO.interface
        and op.character==UnitName("player") and op.realm==GetRealmName()
end
function AO.VerifyActionOperation(op)
    return sameClient(op) and GetCursorInfo()==nil
        and matches(op.target_slot,op.source) and matches(op.source_slot,op.target)
end
function AO.ApplyActionOperation(op)
    assert(type(op)=="table" and op.kind=="move_action_slot" and sameClient(op), "Action client mismatch")
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"), "Action changes unavailable")
    assert(GetCursorInfo()==nil, "Cursor already occupied")
    validateAction(op.source); validateAction(op.target)
    assert(not op.source.empty and op.source_slot~=op.target_slot, "Invalid action move")
    for _,slot in ipairs({op.source_slot,op.target_slot}) do
        assert(type(slot)=="number" and slot%1==0 and slot>=1 and slot<=4096, "Invalid action slot")
    end
    local observed=AO.collectors.action_bar_observation()
    assert(observed.slots[tostring(op.source_slot)] and observed.slots[tostring(op.target_slot)], "Action slot no longer observed")
    assert(matches(op.source_slot,op.source) and matches(op.target_slot,op.target), "Action contents changed")
    -- Caller must checkpoint uncertainty before entry. Never invoke an action body.
    PickupAction(op.source_slot,false)
    assert(cursorMatches(op.source), "Action pickup unconfirmed; cursor retained")
    assert(matches(op.source_slot,{empty=true}) and matches(op.target_slot,op.target), "Action slots changed after pickup")
    PlaceAction(op.target_slot)
    if not op.target.empty then
        assert(matches(op.target_slot,op.source) and cursorMatches(op.target), "Action swap unconfirmed; cursor retained")
        assert(matches(op.source_slot,{empty=true}), "Source occupied; cursor retained")
        PlaceAction(op.source_slot)
    end
    return {kind=op.kind,status=AO.VerifyActionOperation(op) and "applied" or "outcome_unknown"}
end
