use mlua::Lua;

#[test]
fn metadata_changes_require_exact_preimage_and_verified_native_result() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105,collectors={}}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end
        function UnitChannelInfo() end
        function GetCursorInfo() return cursor end
        function copy(t) local out={} for k,v in pairs(t) do out[k]=type(v)=='table' and copy(v) or v end return out end
        function reset()
            combat=false; casting=nil; cursor=nil; writes=0; reject=false; delayed=false; failRead=false
            local slots={} for i=1,19 do slots[tostring(i)]={} end
            slots['16']={raw_item_id=123,raw_location=999,ignored=false,
                observed_item={kind='equipment',slot=16,item_id=123,guid='Item-A',locked=false}}
            sets={['0']={name='Original',icon=134400,slots=slots,assigned_spec=2},
                ['1']={name='Other',icon=134400,slots=copy(slots)}}
            op={kind='modify_equipment_set',family='retail',variant='live',build='70688',interface=120105,
                character='Fixture',realm='Fixture',set_id=0,expected=copy(sets['0']),name='Renamed',icon=123}
        end
        function AO.collectors.equipment_sets()
            if failRead then error('unavailable') end
            return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)}
        end
        C_EquipmentSet={
            GetEquipmentSetID=function(name) for id,set in pairs(sets) do if set.name:lower()==name:lower() then return tonumber(id) end end end,
            ModifyEquipmentSet=function(id,name,icon)
                writes=writes+1; assert(type(icon)=='string')
                if reject then return end
                if delayed then pending=function() sets[tostring(id)].name=name; sets[tostring(id)].icon=tonumber(icon) end
                else sets[tostring(id)].name=name; sets[tostring(id)].icon=tonumber(icon) end
            end,
            DeleteEquipmentSet=function(id) writes=writes+1; if not reject then sets[tostring(id)]=nil end end,
        }
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentSetOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        combat=true; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); combat=false
        casting='cast'; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); casting=nil
        cursor='item'; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); cursor=nil
        op.build='wrong'; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); op.build='70688'
        op.name='other'; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); op.name='Renamed'
        sets['0'].slots['16'].observed_item.guid='Item-B'
        assert(not pcall(AO.ApplyEquipmentSetOperation,op)); sets['0'].slots['16'].observed_item.guid='Item-A'
        sets['0'].assigned_spec=1; assert(not pcall(AO.ApplyEquipmentSetOperation,op)); sets['0'].assigned_spec=2
        local modify=C_EquipmentSet.ModifyEquipmentSet; C_EquipmentSet.ModifyEquipmentSet=nil
        assert(not pcall(AO.ApplyEquipmentSetOperation,op)); C_EquipmentSet.ModifyEquipmentSet=modify
        assert(writes==0)
        assert(AO.ApplyEquipmentSetOperation(op).status=='applied' and writes==1)
        assert(sets['0'].name=='Renamed' and sets['0'].icon==123 and sets['0'].assigned_spec==2)
        assert(sets['1'].name=='Other' and sets['0'].slots['16'].observed_item.guid=='Item-A')
        assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==1)
        reset(); op.name=op.expected.name; op.icon=op.expected.icon
        assert(AO.ApplyEquipmentSetOperation(op).status=='already_selected' and writes==0)
        reset(); reject=true
        assert(AO.ApplyEquipmentSetOperation(op).status=='outcome_unknown' and writes==1)
        reset(); delayed=true
        assert(AO.ApplyEquipmentSetOperation(op).status=='outcome_unknown' and writes==1)
        pending(); assert(AO.VerifyEquipmentSetOperation(op) and writes==1)
        sets['0'].slots['16'].raw_item_id=456; assert(not AO.VerifyEquipmentSetOperation(op))
        reset(); op.kind='delete_equipment_set'
        assert(AO.ApplyEquipmentSetOperation(op).status=='applied' and writes==1 and sets['0']==nil and sets['1'])
        assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==1)
        reset(); op.kind='delete_equipment_set'; reject=true
        assert(AO.ApplyEquipmentSetOperation(op).status=='outcome_unknown' and writes==1)
        reset(); failRead=true; assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==0)
        reset(); issecretvalue=function(v) return v=='Original' end
        assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==0); issecretvalue=nil
        reset(); op.name='bad|name'; assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==0)
        reset(); op.kind='delete_equipment_set'
        local remove=C_EquipmentSet.DeleteEquipmentSet; C_EquipmentSet.DeleteEquipmentSet=nil
        assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==0)
        C_EquipmentSet.DeleteEquipmentSet=remove
        reset()
        C_EquipmentSet.ModifyEquipmentSet=function(id,name,icon)
            writes=writes+1; sets[tostring(id)].name=name; failRead=true
        end
        assert(AO.ApplyEquipmentSetOperation(op).status=='outcome_unknown' and writes==1)
        reset()
        C_EquipmentSet.ModifyEquipmentSet=function() writes=writes+1; error('native call failed') end
        assert(not pcall(AO.ApplyEquipmentSetOperation,op) and writes==1)
    "#).exec()
}
