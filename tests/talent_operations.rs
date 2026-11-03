use mlua::Lua;
#[test]
fn talent_request_requires_preimages_and_committed_readback() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
 AO={family='retail',interface=120105}; current='old'; selected=8; staged=false; writes=0; mode='pending'
 function GetBuildInfo() return '', '70688', '',120105 end
 function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
 function InCombatLockdown() return false end
 function UnitCastingInfo() end; function UnitChannelInfo() end
 C_SpecializationInfo={GetSpecialization=function() return 1 end,GetSpecializationInfo=function() return 263 end}
 Enum={LoadConfigResult={Error=0,NoChangesNecessary=1,LoadInProgress=2,Ready=3}}
 C_Traits={GetConfigInfo=function() return {name='Build',usesSharedActionBars=true} end,
 ConfigHasStagedChanges=function(id) return id==7 and staged end,
 GenerateImportString=function(id) return id==7 and current or 'new' end,
 GetLoadoutSerializationVersion=function() return 2 end}
 C_ClassTalents={GetActiveConfigID=function() return 7 end,GetLastSelectedSavedConfigID=function() return selected end,
 GetConfigIDsBySpecID=function() return {8,9} end,CanEditTalents=function() return true end,
 LoadConfig=function(id,apply) assert(id==9 and apply); writes=writes+1; if mode=='instant' then current='new'; selected=9; return 1 end; return 2 end}
 PlayerSpellsFrame={TalentsFrame={IsShown=function() return true end,IsInspecting=function() return false end,
 IsCommitInProgress=function() return false end,GetConfigID=function() return 7 end,
 SetSelectedSavedConfigID=function(_,id,apply) return C_ClassTalents.LoadConfig(id,apply) end}}
 op={kind='load_talent_config',interface=120105,build='70688',character='Test',realm='Test',spec_id=263,
 serialization_version=2,expected_active_config_id=7,expected_selected_config_id=8,target_config_id=9,
 desired_name='Build',uses_shared_action_bars=true,expected_import_string='old',desired_import_string='new'}
 "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/TalentOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
 staged=true; assert(not pcall(AO.ApplyTalentOperation,op)); staged=false
 current='changed'; assert(not pcall(AO.ApplyTalentOperation,op)); current='old'
 op.desired_name='renamed'; assert(not pcall(AO.ApplyTalentOperation,op)); op.desired_name='Build'
 local frame=PlayerSpellsFrame.TalentsFrame
 frame.IsShown=function() return false end; assert(not pcall(AO.ApplyTalentOperation,op))
 frame.IsShown=function() return true end
 frame.IsCommitInProgress=function() return true end; assert(not pcall(AO.ApplyTalentOperation,op))
 frame.IsCommitInProgress=function() return false end
 assert(writes==0)
 assert(AO.ApplyTalentOperation(op).status=='outcome_unknown' and writes==1)
 assert(not AO.VerifyTalentOperation(op) and writes==1)
 current='new'; staged=true; assert(not AO.VerifyTalentOperation(op)); staged=false
 assert(not AO.VerifyTalentOperation(op)); selected=9
 assert(AO.VerifyTalentOperation(op) and writes==1)
 current='old'; selected=8; mode='instant'
 assert(AO.ApplyTalentOperation(op).status=='applied' and writes==2)
 "#,
    )
    .exec()
}
