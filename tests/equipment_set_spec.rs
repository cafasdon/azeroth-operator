use mlua::Lua;

#[test]
fn associations_bind_catalogue_and_spec_identity_without_multi_call_repair() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105,collectors={}}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end; function UnitChannelInfo() return channel end
        function GetCursorInfo() return cursor end
        function GetNumSpecializations() return 3 end
        C_SpecializationInfo={GetSpecializationInfo=function(i) return i+100 end}
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        function reset()
            writes=0; combat=false; casting=nil; channel=nil; cursor=nil; mode=nil
            local slots={} for i=1,19 do slots[tostring(i)]={} end
            sets={['0']={name='Target',icon=123,assigned_spec=1,slots=copy(slots)},
                ['1']={name='Displaced',icon=123,assigned_spec=2,slots=copy(slots)},
                ['2']={name='Untouched',icon=123,assigned_spec=3,slots=copy(slots)}}
            op={kind='assign_equipment_set_spec',family='retail',variant='live',build='70688',interface=120105,
                character='Fixture',realm='Fixture',set_id=0,spec_index=2,spec_id=102,expected_catalogue=copy(sets)}
            C_EquipmentSet={GetEquipmentSetForSpec=function(spec)
                for id,set in pairs(sets) do if set.assigned_spec==spec then return tonumber(id) end end end,
                AssignSpecToEquipmentSet=function(id,spec)
                    writes=writes+1
                    if mode=='throw' then error('native failure') end
                    if mode=='reject' or mode=='delayed' then return end
                    for key,set in pairs(sets) do if set.assigned_spec==spec and mode~='partial' then set.assigned_spec=nil end end
                    sets[tostring(id)].assigned_spec=spec
                    if mode=='throw_after_effect' then error('native error after effect') end
                    if mode=='read_failure' then C_EquipmentSet.GetEquipmentSetForSpec=function() error('readback failed') end end
                    if mode=='other' then sets['2'].name='Unexpected' end
                end,
                UnassignEquipmentSetSpec=function(id) writes=writes+1; if mode~='reject' then sets[tostring(id)].assigned_spec=nil end end}
        end
        function AO.collectors.equipment_sets()
            return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)}
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentSetSpec.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        assert(AO.ApplyEquipmentSetSpec(op).status=='applied' and writes==1)
        assert(sets['0'].assigned_spec==2 and sets['1'].assigned_spec==nil and sets['2'].assigned_spec==3)
        assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==1)
        reset(); op.spec_index=1; op.spec_id=101
        assert(AO.ApplyEquipmentSetSpec(op).status=='already_selected' and writes==0)
        reset(); op.kind='unassign_equipment_set_spec'; op.spec_index=nil; op.spec_id=nil
        assert(AO.ApplyEquipmentSetSpec(op).status=='applied' and writes==1)
        assert(sets['0'].assigned_spec==nil and sets['1'].assigned_spec==2)
        op.expected_catalogue=copy(sets)
        assert(AO.ApplyEquipmentSetSpec(op).status=='already_selected' and writes==1)
        for _,m in ipairs({'reject','partial','other','read_failure','delayed'}) do
            reset(); mode=m; assert(AO.ApplyEquipmentSetSpec(op).status=='outcome_unknown' and writes==1,m)
        end
        sets['0'].assigned_spec=2; sets['1'].assigned_spec=nil
        assert(AO.VerifyEquipmentSetSpec(op) and writes==1)
        reset(); mode='throw'; assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==1)
        reset(); mode='throw_after_effect'
        assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==1)
        assert(sets['0'].assigned_spec==2 and sets['1'].assigned_spec==nil)
        assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==1)
        local refusals={
            function() combat=true end,function() casting='cast' end,function() channel='channel' end,
            function() cursor='item' end,function() op.spec_id=999 end,function() op.spec_index=4 end,
            function() op.family='forever' end,function() op.build='wrong' end,
            function() sets['1'].name='Player edit' end,function() sets['3']=copy(sets['0']); sets['3'].assigned_spec=nil end,
            function() C_EquipmentSet.GetEquipmentSetForSpec=function() return nil end end,
            function() C_EquipmentSet.AssignSpecToEquipmentSet=nil end,
            function() op.kind='unassign_equipment_set_spec'; op.spec_index=nil; op.spec_id=nil; C_EquipmentSet.UnassignEquipmentSetSpec=nil end,
            function() op.expected_catalogue['2'].assigned_spec=2 end,
        }
        for i,f in ipairs(refusals) do reset(); f(); assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==0,'guard '..i) end
        reset(); issecretvalue=function(v) return v=='Target' end
        assert(not pcall(AO.ApplyEquipmentSetSpec,op) and writes==0); issecretvalue=nil
    "#).exec()
}
