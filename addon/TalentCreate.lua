-- Source-only current-build creation. Native UI owns asynchronous selection.
local _,AO=...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v),"Restricted talent creation state")
    return v
end
local function validate(op)
    local _,build,_,interface=GetBuildInfo()
    assert(type(op)=="table" and op.kind=="create_talent_config" and AO.family=="retail" and op.family==AO.family
        and op.variant==AO.variant and interface==AO.interface and interface==op.interface and tostring(build)==op.build
        and UnitName('player')==op.character and GetRealmName()==op.realm,"Talent creation client mismatch")
    assert(type(op.expected)=="table" and type(op.expected.entries)=="table","Talent catalogue required")
    assert(type(op.name)=="string" and #op.name>0 and #op.name<=64 and not op.name:find("[%c|]")
        and not op.name:match("^%s") and not op.name:match("%s$"),"Invalid new loadout name")
    assert(type(op.expected_new_shared_action_bars)=="boolean","Expected new shared-bars state required")
    local count=0
    for _,entry in pairs(op.expected.entries) do
        count=count+1; assert(entry.name:lower()~=op.name:lower(),"Loadout name collision")
    end
    assert(count<128,"Captured loadout catalogue full")
end
function AO.VerifyTalentCreate(op)
    validate(op)
    if GetCursorInfo()~=nil then return false end
    local current=AO.ReadTalentCatalogueState()
    local newID
    for id,entry in pairs(current.entries) do
        if not op.expected.entries[id] then
            if newID then return false end
            newID=tonumber(id)
            if entry.name~=op.name or entry.import_string~=op.expected.active_import_string
                or entry.serialization_version~=op.expected.serialization_version
                or entry.uses_shared_action_bars~=op.expected_new_shared_action_bars then return false end
        end
    end
    if not newID or current.selected_config_id~=newID then return false end
    local desired={}
    for key,value in pairs(op.expected) do desired[key]=value end
    desired.selected_config_id=newID; desired.entries={}
    for id,entry in pairs(op.expected.entries) do desired.entries[id]=entry end
    desired.entries[tostring(newID)]={name=op.name,import_string=op.expected.active_import_string,
        serialization_version=op.expected.serialization_version,uses_shared_action_bars=op.expected_new_shared_action_bars}
    return AO.TalentCatalogueStatesEqual(current,desired)
end
function AO.ApplyTalentCreate(op)
    validate(op)
    assert(not InCombatLockdown() and not UnitCastingInfo('player') and not UnitChannelInfo('player') and GetCursorInfo()==nil,"Talent creation unavailable")
    assert(C_ClassTalents and type(C_ClassTalents.RequestNewConfig)=="function"
        and read(C_ClassTalents.CanCreateNewConfig())==true and read(C_ClassTalents.CanEditTalents())==true,"Cannot create talent loadout")
    local frame=PlayerSpellsFrame and PlayerSpellsFrame.TalentsFrame
    assert(frame and read(frame:IsShown())==true and read(frame:IsInspecting())==false and read(frame:IsCommitInProgress())==false
        and frame:GetConfigID()==op.expected.active_config_id and type(frame.OnTraitConfigCreateStarted)=="function"
        and type(frame.HasAnyPurchasedRanks)=="function","Open idle player Talents panel")
    assert(read(frame.autoLoadNewConfigID)==nil and not read(frame.nextNewConfigRequiresPopulatedCheck)
        and not read(frame.areClassTalentCommitVisualsActive),"Another talent creation or commit is pending")
    for _,dialog in pairs({edit=ClassTalentLoadoutEditDialog,create=ClassTalentLoadoutCreateDialog,import=ClassTalentLoadoutImportDialog}) do
        assert(read(dialog:IsShown())==false,"Close loadout dialogs")
    end
    assert(type(StaticPopup_Visible)=="function" and not read(StaticPopup_Visible('LOADOUT_CONFIRM_DELETE_DIALOG'))
        and not read(StaticPopup_Visible('LOADOUT_CONFIRM_SHARED_ACTION_BARS')) and not read(StaticPopup_Visible('LOADOUT_IMPORT_ERROR_DIALOG')),"Close loadout confirmation")
    local purchased=read(frame:HasAnyPurchasedRanks()); assert(type(purchased)=="boolean","Purchased-rank state unavailable")
    assert(AO.TalentCatalogueStatesEqual(AO.ReadTalentCatalogueState(),op.expected),"Talent state changed")
    -- Mirrors native NewEntryCallback. No guessed ID, direct commit or retry.
    local accepted=read(C_ClassTalents.RequestNewConfig(op.name))
    if accepted==true then frame:OnTraitConfigCreateStarted(purchased) end
    local ok,verified=pcall(AO.VerifyTalentCreate,op)
    return {kind=op.kind,status=ok and verified and 'applied' or 'outcome_unknown'}
end
