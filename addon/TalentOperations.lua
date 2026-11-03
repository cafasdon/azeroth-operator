local _, AO = ...
local function specID()
    return C_SpecializationInfo.GetSpecializationInfo(C_SpecializationInfo.GetSpecialization())
end
local function targetMatches(op)
    local info=C_Traits.GetConfigInfo(op.target_config_id)
    return info and info.name==op.desired_name and info.usesSharedActionBars==op.uses_shared_action_bars
        and not C_Traits.ConfigHasStagedChanges(op.target_config_id)
        and C_Traits.GenerateImportString(op.target_config_id)==op.desired_import_string
end
-- Observation only: safe to use for later completion reconciliation, never commits.
function AO.VerifyTalentOperation(op)
    local _,build,_,interface=GetBuildInfo()
    if AO.family~="retail" or interface~=op.interface or tostring(build)~=op.build
        or UnitName("player")~=op.character or GetRealmName()~=op.realm or specID()~=op.spec_id then return false end
    local active=C_ClassTalents.GetActiveConfigID()
    return C_ClassTalents.GetLastSelectedSavedConfigID(op.spec_id)==op.target_config_id
        and active==op.expected_active_config_id and C_Traits.GetLoadoutSerializationVersion()==op.serialization_version
        and not C_Traits.ConfigHasStagedChanges(active) and targetMatches(op)
        and C_Traits.GenerateImportString(active)==op.desired_import_string
end
function AO.ApplyTalentOperation(op)
    assert(type(op)=="table" and op.kind=="load_talent_config","Invalid talent operation")
    local _,build,_,interface=GetBuildInfo()
    assert(AO.family=="retail" and interface==AO.interface and interface==op.interface and tostring(build)==op.build,"Client changed")
    assert(UnitName("player")==op.character and GetRealmName()==op.realm,"Character changed")
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"),"Talent change unavailable in current state")
    assert(specID()==op.spec_id,"Specialisation changed")
    assert(C_Traits.GetLoadoutSerializationVersion()==op.serialization_version,"Talent serialization changed")
    local active=C_ClassTalents.GetActiveConfigID()
    assert(active==op.expected_active_config_id,"Working configuration changed")
    assert((C_ClassTalents.GetLastSelectedSavedConfigID(op.spec_id) or 0)==op.expected_selected_config_id,"Selected loadout changed")
    assert(not C_Traits.ConfigHasStagedChanges(active),"Unapplied player talent changes")
    assert(C_Traits.GenerateImportString(active)==op.expected_import_string,"Current talent contents changed")
    local found=false
    for _,id in ipairs(C_ClassTalents.GetConfigIDsBySpecID(op.spec_id)) do if id==op.target_config_id then found=true end end
    assert(found and targetMatches(op),"Saved loadout changed or unavailable")
    local allowed,reason=C_ClassTalents.CanEditTalents()
    assert(allowed,reason or "Talent editing unavailable")
    -- The shared caller must checkpoint uncertainty before this one request.
    local result,detail=C_ClassTalents.LoadConfig(op.target_config_id,true)
    assert(result~=Enum.LoadConfigResult.Error,detail or "Talent load refused")
    -- Ready/LoadInProgress are not completion. No CommitConfig or automatic retry.
    local ok,verified=pcall(AO.VerifyTalentOperation,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
