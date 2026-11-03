-- One explicit player-reviewed key change; never invokes the bound action.
local _, AO = ...
function AO.ApplyBindingOperation(op)
    assert(AO.family=="retail", "Binding operations unavailable for this family")
    assert(not InCombatLockdown(), "Binding changes unavailable in combat")
    local _,_,_,interface=GetBuildInfo()
    assert(interface==AO.interface, "Client interface changed")
    assert(type(op)=="table" and op.kind=="set_binding", "Invalid binding operation")
    assert(type(op.key)=="string" and #op.key>0 and #op.key<=128 and not op.key:find("[%z\1-\31]"), "Invalid key")
    for _,value in ipairs({op.expected,op.desired}) do
        assert(type(value)=="string" and #value<=256 and not value:find("[%z\1-\31]"), "Invalid action")
    end
    assert(type(op.expected)=="string" and type(op.desired)=="string", "Missing action")
    assert(op.binding_set==1 or op.binding_set==2, "Invalid binding set")
    assert(type(op.context)=="number" and op.context>=0 and op.context<=9 and op.context%1==0, "Unsupported binding context")
    assert(GetCurrentBindingSet and GetBindingAction and SetBinding and SaveBindings, "Binding API unavailable")
    assert(GetCurrentBindingSet()==op.binding_set, "Binding set changed")
    assert(C_KeyBindings and C_KeyBindings.GetBindingContextForAction, "Binding context API unavailable")
    if op.desired~="" then
        local context=C_KeyBindings.GetBindingContextForAction(op.desired) or 0
        assert(context==op.context, "Action belongs to a different context")
        assert(C_KeyBindings.GetBindingIndex and C_KeyBindings.GetBindingIndex(op.desired), "Action not registered")
    end
    assert(GetBindingAction(op.key,false,op.context)==op.expected, "Key binding changed")
    if op.expected==op.desired then return {status="already_selected",kind=op.kind} end
    -- Caller must record outcome_unknown before invoking this function. A failure
    -- can leave an active change even if SaveBindings has not completed.
    assert(SetBinding(op.key,op.desired~="" and op.desired or nil,op.context), "SetBinding rejected")
    assert(GetBindingAction(op.key,false,op.context)==op.desired, "Binding readback mismatch")
    assert(GetCurrentBindingSet()==op.binding_set, "Binding set changed before save")
    SaveBindings(op.binding_set)
    assert(GetBindingAction(op.key,false,op.context)==op.desired, "Saved binding readback mismatch")
    return {status="applied",kind=op.kind,persistence="save_requested"}
end
