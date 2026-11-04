use mlua::Lua;

#[test]
fn saved_loadout_metadata_preserves_active_contents_and_bystanders() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105}
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() end; function UnitChannelInfo() end; function GetCursorInfo() return cursor end
        function StaticPopup_Visible() return popup end
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        function finish()
            if op.kind=='rename_talent_config' then entries['9'].name=op.name else entries['9']=nil end
        end
        function native(id,name)
            assert(id==9); writes=writes+1
            if mode=='throw_before' then error('before') end
            if mode=='delay' or mode=='reject' then return true end
            finish()
            if mode=='throw_after' then error('after') end
            if mode=='active' then current='Q0hBTkdFRA==' end
            if mode=='bystander' then entries['8'].name='Changed' end
            if mode=='selection' then selected=nil end
            if mode=='shared' and entries['9'] then entries['9'].uses_shared_action_bars=false end
            return false
        end
        function reset(kind)
            spec=263; active=7; selected=8; current='QUJDRA=='; version=2; mode=nil; staged=nil
            combat=false; cursor=nil; popup=nil; visible=true; inspect=false; committing=false; writes=0
            entries={['8']={name='Other',uses_shared_action_bars=false,import_string='RUZHSA==',serialization_version=2},
                ['9']={name='Build',uses_shared_action_bars=true,import_string='SUo=',serialization_version=2}}
            op={kind=kind or 'rename_talent_config',family='retail',variant='live',interface=120105,build='70688',
                character='Fixture',realm='Fixture',target_config_id=9,name=kind~='delete_talent_config' and 'Renamed' or nil,
                expected={spec_id=spec,active_config_id=active,selected_config_id=selected,serialization_version=version,
                    active_import_string=current,entries=copy(entries)}}
            C_SpecializationInfo={GetSpecialization=function() return 1 end,GetSpecializationInfo=function() return spec end}
            C_ClassTalents={GetActiveConfigID=function() return active end,GetLastSelectedSavedConfigID=function() return selected end,
                GetConfigIDsBySpecID=function()
                    if mode=='sparse' then return {[1]=8,[3]=9} end
                    if mode=='duplicate' then return {8,8} end
                    local r={} for id in pairs(entries) do r[#r+1]=tonumber(id) end; table.sort(r); return r
                end,CanEditTalents=function() return true end,RenameConfig=native,DeleteConfig=native}
            C_Traits={GetLoadoutSerializationVersion=function() return version end,
                ConfigHasStagedChanges=function(id) return id==staged end,
                GenerateImportString=function(id) return id==7 and current or entries[tostring(id)].import_string end,
                GetConfigInfo=function(id)
                    local e=entries[tostring(id)]; if not e then return end
                    return {name=e.name,usesSharedActionBars=e.uses_shared_action_bars}
                end}
            PlayerSpellsFrame={TalentsFrame={IsShown=function() return visible end,IsInspecting=function() return inspect end,
                IsCommitInProgress=function() return committing end,GetConfigID=function() return 7 end}}
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/TalentMetadata.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        for _,kind in ipairs({'rename_talent_config','delete_talent_config'}) do
            reset(kind); assert(AO.ApplyTalentMetadata(op).status=='applied' and writes==1)
            assert(not pcall(AO.ApplyTalentMetadata,op) and writes==1)
            for _,failure in ipairs({'delay','reject','active','bystander','selection'}) do
                reset(kind); mode=failure
                assert(AO.ApplyTalentMetadata(op).status=='outcome_unknown' and writes==1,failure)
            end
            reset(kind); mode='delay'; assert(AO.ApplyTalentMetadata(op).status=='outcome_unknown')
            finish(); assert(AO.VerifyTalentMetadata(op) and writes==1)
            for _,failure in ipairs({'throw_before','throw_after'}) do
                reset(kind); mode=failure; assert(not pcall(AO.ApplyTalentMetadata,op) and writes==1)
            end
            for _,failure in ipairs({'sparse','duplicate','combat','cursor','popup','hidden','inspect','committing','staged','staged_saved','spec','version','stale','missing_api'}) do
                reset(kind)
                if failure=='combat' then combat=true elseif failure=='cursor' then cursor='item'
                elseif failure=='popup' then popup=true elseif failure=='hidden' then visible=false
                elseif failure=='inspect' then inspect=true elseif failure=='committing' then committing=true
                elseif failure=='staged' then staged=7 elseif failure=='staged_saved' then staged=8
                elseif failure=='spec' then spec=264 elseif failure=='version' then version=3
                elseif failure=='stale' then entries['8'].import_string='TkVX'
                elseif failure=='missing_api' then C_ClassTalents.RenameConfig=nil; C_ClassTalents.DeleteConfig=nil
                else mode=failure end
                assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0,failure)
            end
        end
        reset(); op.name='Build'; assert(AO.ApplyTalentMetadata(op).status=='already_selected' and writes==0)
        reset(); selected=9; op.expected.selected_config_id=9
        assert(AO.ApplyTalentMetadata(op).status=='applied' and writes==1)
        reset('delete_talent_config'); selected=9; op.expected.selected_config_id=9
        assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0)
        reset(); op.name='OTHER'; assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0)
        reset(); mode='shared'; assert(AO.ApplyTalentMetadata(op).status=='outcome_unknown' and writes==1)
        reset(); local reads=0; C_ClassTalents.GetLastSelectedSavedConfigID=function()
            reads=reads+1; return reads==1 and 8 or 9 end
        assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0)
        reset(); function issecretvalue(v) return v=='Build' end
        assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0); issecretvalue=nil
        reset(); AO.family='classic'; assert(not pcall(AO.ApplyTalentMetadata,op) and writes==0)
    "#).exec()
}
