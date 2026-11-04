-- Source-only talent import. Native UI owns asynchronous selection.
local _,AO=...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v),"Restricted talent creation state")
    return v
end
local contexts=setmetatable({}, {__mode="k"})
local function copy(value)
    value=read(value)
    if type(value)~="table" then return value end
    local result={} for k,v in pairs(value) do result[k]=copy(v) end return result
end
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and op.kind=="import_talent_config" and AO.family=="retail" and op.family==AO.family
        and op.variant==AO.variant and interface==AO.interface and interface==op.interface and tostring(build)==op.build
        and UnitName('player')==op.character and GetRealmName()==op.realm,"Talent creation client mismatch")
    assert(type(op.expected)=="table" and type(op.expected.entries)=="table","Talent catalogue required")
    assert(type(op.name)=="string" and #op.name>0 and #op.name<=64 and not op.name:find("[%c|]")
        and not op.name:match("^%s") and not op.name:match("%s$"),"Invalid new loadout name")
    assert(type(op.expected_new_shared_action_bars)=="boolean","Expected new shared-bars state required")
    assert(type(op.import_string)=="string" and #op.import_string>0 and #op.import_string<=16384
        and not op.import_string:find("[^A-Za-z0-9+/]"),"Invalid talent import")
    assert(type(op.allow_empty_hash)=="boolean","Explicit empty-hash choice required")
    local count=0
    for _,entry in pairs(op.expected.entries) do
        count=count+1; assert(entry.name:lower()~=op.name:lower(),"Loadout name collision")
    end
    assert(count<128,"Captured loadout catalogue full")
end
function AO.VerifyTalentImport(op)
    validate(op)
    local context=contexts[op]
    if not context or not AO.TalentCatalogueStatesEqual(op,context.operation) or GetCursorInfo()~=nil then return false end
    local current=AO.ReadTalentCatalogueState()
    local newID
    for id,entry in pairs(current.entries) do
        if not op.expected.entries[id] then
            if newID then return false end
            newID=tonumber(id)
            if entry.name~=op.name or not AO.TalentImportContentsMatch(context.plan,entry.import_string)
                or entry.serialization_version~=op.expected.serialization_version
                or entry.uses_shared_action_bars~=op.expected_new_shared_action_bars then return false end
        end
    end
    if not newID or current.selected_config_id~=newID then return false end
    if not AO.TalentImportContentsMatch(context.plan,current.active_import_string) then return false end
    local frame=PlayerSpellsFrame and PlayerSpellsFrame.TalentsFrame
    if not frame or read(frame.lastSelectedConfigID)~=newID or not frame.LoadSystem
        or read(frame.LoadSystem:GetSelectionID())~=newID then return false end
    local desired={}
    for key,value in pairs(op.expected) do desired[key]=value end
    desired.active_import_string=current.active_import_string
    desired.selected_config_id=newID; desired.entries={}
    for id,entry in pairs(op.expected.entries) do desired.entries[id]=entry end
    desired.entries[tostring(newID)]={name=op.name,import_string=current.entries[tostring(newID)].import_string,
        serialization_version=op.expected.serialization_version,uses_shared_action_bars=op.expected_new_shared_action_bars}
    return AO.TalentCatalogueStatesEqual(current,desired)
        and AO.TalentCatalogueStatesEqual(current,AO.ReadTalentCatalogueState())
end
function AO.ApplyTalentImport(op)
    validate(op)
    assert(not contexts[op],"Import already attempted; reconcile before retry")
    assert(not InCombatLockdown() and not UnitCastingInfo('player') and not UnitChannelInfo('player') and GetCursorInfo()==nil,"Talent creation unavailable")
    assert(C_ClassTalents and type(C_ClassTalents.ImportLoadout)=="function"
        and read(C_ClassTalents.CanCreateNewConfig())==true and read(C_ClassTalents.CanEditTalents())==true,"Cannot create talent loadout")
    local frame=PlayerSpellsFrame and PlayerSpellsFrame.TalentsFrame
    assert(frame and read(frame:IsShown())==true and read(frame:IsInspecting())==false and read(frame:IsCommitInProgress())==false
        and frame:GetConfigID()==op.expected.active_config_id and type(frame.OnTraitConfigCreateStarted)=="function"
        and frame.LoadSystem and type(frame.LoadSystem.GetSelectionID)=="function","Open idle player Talents panel")
    assert(read(frame.autoLoadNewConfigID)==nil and not read(frame.nextNewConfigRequiresPopulatedCheck)
        and not read(frame.areClassTalentCommitVisualsActive),"Another talent creation or commit is pending")
    for _,dialog in pairs({edit=ClassTalentLoadoutEditDialog,create=ClassTalentLoadoutCreateDialog,import=ClassTalentLoadoutImportDialog}) do
        assert(read(dialog:IsShown())==false,"Close loadout dialogs")
    end
    assert(type(StaticPopup_Visible)=="function" and not read(StaticPopup_Visible('LOADOUT_CONFIRM_DELETE_DIALOG'))
        and not read(StaticPopup_Visible('LOADOUT_CONFIRM_SHARED_ACTION_BARS')) and not read(StaticPopup_Visible('LOADOUT_IMPORT_ERROR_DIALOG')),"Close loadout confirmation")
    local plan=AO.PlanTalentImport(op.import_string,op.allow_empty_hash)
    assert(plan.conversion_verified==true and plan.observation.active_config_id==op.expected.active_config_id
        and plan.observation.spec_id==op.expected.spec_id and plan.observation.serialization_version==op.expected.serialization_version,
        "Import plan does not match captured talents")
    assert(AO.TalentCatalogueStatesEqual(AO.ReadTalentCatalogueState(),op.expected),"Talent state changed")
    -- Retain independent preimage/plan before the single native mutation. Native
    -- code receives a copy so it cannot alter our subsequent readback criteria.
    contexts[op]={operation=copy(op),plan=copy(plan)}
    local accepted=read(C_ClassTalents.ImportLoadout(op.expected.active_config_id,copy(plan.entries),op.name,op.import_string))
    if accepted==true then frame:OnTraitConfigCreateStarted(#plan.entries>0) end
    local ok,verified=pcall(AO.VerifyTalentImport,op)
    return {kind=op.kind,status=ok and verified and 'applied' or 'outcome_unknown'}
end
