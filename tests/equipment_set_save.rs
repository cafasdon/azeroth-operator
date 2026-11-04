use mlua::Lua;

#[test]
fn create_and_save_require_exact_preimages_and_complete_readback() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105,collectors={}}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end
        function UnitChannelInfo() return channel end
        function GetCursorInfo() return cursor end
        function IsInventoryItemLocked(i) return locked==i end
        function StaticPopup_Visible() return popup end
        function copy(t) local out={} for k,v in pairs(t) do out[k]=type(v)=='table' and copy(v) or v end return out end
        ItemLocation={CreateFromEquipmentSlot=function(_,i) return i end}
        C_Item={DoesItemExist=function(i) return worn[tostring(i)].guid~='' end,
            GetItemGUID=function(i) local g=worn[tostring(i)].guid; if g~='' then return g end end,
            GetItemID=function(i) local id=worn[tostring(i)].item_id; if id~=0 then return id end end}
        function desiredSet()
            local slots={}
            for i=1,19 do
                local k=tostring(i); slots[k]={ignored=op.ignored_slots[k]}
                if not slots[k].ignored then
                    slots[k].raw_item_id=worn[k].item_id
                    slots[k].observed_item={kind='equipment',slot=i,item_id=worn[k].item_id,guid=worn[k].guid,locked=false}
                end
            end
            local old=sets[tostring(op.set_id)]
            return {name=op.name,icon=op.icon,slots=slots,num_ignored=18,num_items=1,
                num_equipped=1,num_in_inventory=0,num_lost=0,assigned_spec=old and old.assigned_spec}
        end
        function finish()
            local id=op.kind=='create_equipment_set' and 0 or op.set_id
            sets[tostring(id)]=desiredSet()
        end
        function native(a,b)
            calls=calls+1
            if op.kind=='create_equipment_set' then assert(a==op.name) else assert(a==op.set_id) end
            assert(b==tostring(op.icon))
            for i=1,19 do assert(flags[i]==op.ignored_slots[tostring(i)]) end
            if mode=='throw_before' then error('before effect') end
            if mode=='delay' or mode=='reject' then return true end
            finish()
            if mode=='throw_after' then error('after effect') end
            if mode=='wrong_guid' then sets[tostring(op.set_id or 0)].slots['16'].observed_item.guid='wrong' end
            if mode=='bystander' then sets['8'].icon=44 end
            if mode=='new_bystander' then sets['9']=copy(sets['8']) end
            if mode=='missing_bystander' then sets['8']=nil end
            if mode=='wrong_spec' then sets[tostring(op.set_id or 0)].assigned_spec=3 end
            if mode=='wrong_count' then sets[tostring(op.set_id or 0)].num_lost=1 end
            if mode=='wrong_ignore' then sets[tostring(op.set_id or 0)].slots['19'].ignored=false end
            if mode=='worn_changed' then worn['16'].guid='changed' end
            if mode=='cursor' then cursor='item' end
            return false
        end
        function reset(kind)
            combat=false; casting=nil; channel=nil; cursor=nil; locked=nil; popup=nil; mode=nil
            calls=0; flagWrites=0; flags={}; worn={}; MAX_EQUIPMENT_SETS_PER_PLAYER=20
            PaperDollFrame={EquipmentManagerPane={IsShown=function() return editor end}}; editor=false
            GearManagerPopupFrame=nil
            local ignored={}
            for i=1,19 do
                flags[i]=false; ignored[tostring(i)]=i~=16; worn[tostring(i)]={guid='',item_id=0}
            end
            worn['16']={guid='Item-weapon',item_id=123}
            sets={['8']={name='Other',icon=99,slots={}}}
            op={kind=kind or 'create_equipment_set',family='retail',variant='live',build='70688',interface=120105,
                character='Fixture',realm='Fixture',name='New',icon=134400,ignored_slots=ignored,equipment_before=copy(worn)}
            if kind=='save_equipment_set' then
                op.set_id=2; sets['2']=desiredSet(); sets['2'].icon=1; sets['2'].assigned_spec=2
            end
            op.expected_catalogue=copy(sets)
            C_EquipmentSet={CreateEquipmentSet=native,SaveEquipmentSet=native,
                GetEquipmentSetID=function(name)
                    for id,entry in pairs(sets) do if entry.name==name then return tonumber(id) end end
                end,
                IsSlotIgnoredForSave=function(i) return flags[i] end,
                IgnoreSlotForSave=function(i)
                    flagWrites=flagWrites+1; flags[i]=true
                    if mode=='setup_race' then worn['16'].guid='changed' end
                end,
                UnignoreSlotForSave=function(i) flagWrites=flagWrites+1; flags[i]=false end}
        end
        function AO.collectors.equipment_sets()
            if mode=='readback_error' and calls>0 then error('unavailable') end
            return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)}
        end
        function restored() for i=1,19 do assert(flags[i]==false) end end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentSetSaveState.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/EquipmentSetSave.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        for _,kind in ipairs({'create_equipment_set','save_equipment_set'}) do
            reset(kind); assert(AO.ApplyEquipmentSetSave(op).status=='applied' and calls==1); restored()
            assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==1)
            for _,value in ipairs({'delay','reject','wrong_guid','bystander','new_bystander','missing_bystander',
                'wrong_spec','wrong_count','wrong_ignore','worn_changed','cursor','readback_error'}) do
                reset(kind); mode=value
                assert(AO.ApplyEquipmentSetSave(op).status=='outcome_unknown',value)
                assert(calls==1); restored()
            end
            reset(kind); mode='delay'; assert(AO.ApplyEquipmentSetSave(op).status=='outcome_unknown')
            finish(); assert(AO.VerifyEquipmentSetSave(op) and calls==1); restored()
            for _,value in ipairs({'throw_before','throw_after'}) do
                reset(kind); mode=value; assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==1); restored()
            end
            reset(kind); mode='setup_race'
            assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==0); restored()
            for _,value in ipairs({'combat','editor','popup','locked','cursor','stale','empty','client'}) do
                reset(kind)
                if value=='combat' then combat=true elseif value=='editor' then editor=true
                elseif value=='popup' then popup={} elseif value=='locked' then locked=16
                elseif value=='cursor' then cursor='item' elseif value=='stale' then sets['8'].icon=1
                elseif value=='empty' then op.ignored_slots['19']=false
                elseif value=='client' then op.build='wrong' end
                assert(not pcall(AO.ApplyEquipmentSetSave,op),value); assert(calls==0 and flagWrites==0)
            end
        end
        reset(); MAX_EQUIPMENT_SETS_PER_PLAYER=1
        assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==0 and flagWrites==0)
        reset(); sets['8'].name=op.name; op.expected_catalogue=copy(sets)
        assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==0 and flagWrites==0)
        reset(); C_EquipmentSet.CreateEquipmentSet=nil
        assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==0 and flagWrites==0)
        reset('save_equipment_set'); sets['2']=desiredSet(); op.expected_catalogue=copy(sets)
        assert(AO.ApplyEquipmentSetSave(op).status=='already_selected' and calls==0 and flagWrites==0)
        reset('save_equipment_set'); op.name='Renamed'
        assert(not pcall(AO.ApplyEquipmentSetSave,op) and calls==0 and flagWrites==0)
    "#).exec()
}
