-- Saved-loadout metadata executor. Caller checkpoints uncertainty first.
local _,AO=...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted talent state")
    return v
end
local function number(v,low)
    assert(type(read(v))=="number" and v%1==0 and v>=low and v<=4294967295, "Invalid talent identity")
    return v
end
local function same(a,b,depth)
    a,b=read(a),read(b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    assert((depth or 0)<6, "Talent state too deep")
    for k,v in pairs(a) do if not same(v,b[k],(depth or 0)+1) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function contents(id)
    assert(read(C_Traits.ConfigHasStagedChanges(id))==false, "Unapplied talent changes")
    local text=read(C_Traits.GenerateImportString(id))
    assert(type(text)=="string" and #text>0 and #text<=16384 and not text:find("[^A-Za-z0-9+/=]"), "Talent contents unavailable")
    return text
end
local function state()
    local index=number(C_SpecializationInfo.GetSpecialization(),1)
    local spec=number(C_SpecializationInfo.GetSpecializationInfo(index),1)
    local active=number(C_ClassTalents.GetActiveConfigID(),1)
    local selected=read(C_ClassTalents.GetLastSelectedSavedConfigID(spec))
    if selected~=nil then number(selected,1) end
    local version=number(C_Traits.GetLoadoutSerializationVersion(),0)
    local ids=read(C_ClassTalents.GetConfigIDsBySpecID(spec))
    assert(type(ids)=="table" and #ids<=128, "Saved loadouts unavailable")
    local count=0
    for key in pairs(ids) do
        count=count+1
        assert(type(key)=="number" and key%1==0 and key>=1 and key<=#ids, "Sparse loadout list")
    end
    assert(count==#ids, "Incomplete loadout list")
    local result={spec_id=spec,active_config_id=active,selected_config_id=selected,
        serialization_version=version,active_import_string=contents(active),entries={}}
    for _,id in ipairs(ids) do
        number(id,1); local key=tostring(id)
        assert(not result.entries[key] and id~=active, "Ambiguous saved loadout catalogue")
        local info=read(C_Traits.GetConfigInfo(id))
        assert(type(info)=="table", "Missing saved loadout")
        local name,shared=read(info.name),read(info.usesSharedActionBars)
        assert(type(name)=="string" and #name>0 and #name<=1024 and type(shared)=="boolean", "Invalid loadout metadata")
        result.entries[key]={name=name,uses_shared_action_bars=shared,import_string=contents(id),serialization_version=version}
    end
    return result
end
local function stableState()
    local first=state()
    assert(same(first,state()), "Talent state changed during observation")
    return first
end
-- Read-only primitives shared by talent creation; no mutation authority here.
AO.ReadTalentCatalogueState=stableState
AO.TalentCatalogueStatesEqual=same
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and (op.kind=="rename_talent_config" or op.kind=="delete_talent_config")
        and AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and interface==AO.interface and interface==op.interface and tostring(build)==op.build
        and UnitName("player")==op.character and GetRealmName()==op.realm, "Talent client mismatch")
    number(op.target_config_id,1)
    assert(type(op.expected)=="table" and type(op.expected.entries)=="table"
        and op.expected.entries[tostring(op.target_config_id)], "Captured loadout required")
    assert(op.target_config_id~=op.expected.active_config_id, "Cannot modify working config")
    if op.kind=="rename_talent_config" then
        assert(type(op.name)=="string" and #op.name>0 and #op.name<=64 and not op.name:find("[%z\1-\31\127|]")
            and not op.name:match("^%s") and not op.name:match("%s$"), "Invalid loadout name")
        for id,entry in pairs(op.expected.entries) do
            assert(id==tostring(op.target_config_id) or entry.name:lower()~=op.name:lower(), "Loadout name already exists")
        end
    else
        assert(op.name==nil, "Delete cannot rename")
        assert(op.target_config_id~=op.expected.selected_config_id, "Selected loadout deletion not yet qualified")
    end
end
local function desired(op)
    local result={}
    for k,v in pairs(op.expected) do result[k]=v end
    result.entries={}
    for id,entry in pairs(op.expected.entries) do
        if id~=tostring(op.target_config_id) then result.entries[id]=entry
        elseif op.kind=="rename_talent_config" then
            local replacement={}
            for k,v in pairs(entry) do replacement[k]=v end
            replacement.name=op.name; result.entries[id]=replacement
        end
    end
    return result
end
function AO.VerifyTalentMetadata(op)
    validate(op)
    if GetCursorInfo()~=nil then return false end
    return same(stableState(),desired(op))
end
function AO.ApplyTalentMetadata(op)
    validate(op)
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player")
        and GetCursorInfo()==nil, "Talent metadata change unavailable")
    local fn
    if C_ClassTalents then
        if op.kind=="rename_talent_config" then fn=C_ClassTalents.RenameConfig else fn=C_ClassTalents.DeleteConfig end
    end
    assert(type(fn)=="function", "Talent metadata API unavailable")
    local frame=PlayerSpellsFrame and PlayerSpellsFrame.TalentsFrame
    assert(frame and read(frame:IsShown())==true and read(frame:IsInspecting())==false
        and read(frame:IsCommitInProgress())==false and frame:GetConfigID()==op.expected.active_config_id,
        "Open the idle player Talents panel first")
    for _,dialog in pairs({edit=ClassTalentLoadoutEditDialog,create=ClassTalentLoadoutCreateDialog,import=ClassTalentLoadoutImportDialog}) do
        assert(read(dialog:IsShown())==false, "Close the loadout editor first")
    end
    assert(type(StaticPopup_Visible)=="function" and not read(StaticPopup_Visible("LOADOUT_CONFIRM_DELETE_DIALOG"))
        and not read(StaticPopup_Visible("LOADOUT_CONFIRM_SHARED_ACTION_BARS")), "Close the loadout confirmation first")
    assert(read(C_ClassTalents.CanEditTalents())==true, "Talent editing unavailable")
    assert(same(stableState(),op.expected), "Talent state changed")
    if same(op.expected,desired(op)) then return {kind=op.kind,status="already_selected"} end
    if op.kind=="rename_talent_config" then fn(op.target_config_id,op.name) else fn(op.target_config_id) end
    local ok,verified=pcall(AO.VerifyTalentMetadata,op)
    return {kind=op.kind,status=ok and verified and "applied" or "outcome_unknown"}
end
