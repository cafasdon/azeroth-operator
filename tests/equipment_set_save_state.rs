use mlua::Lua;

#[test]
fn save_slot_state_is_restored_without_repeating_save() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='retail'}
        function reset()
            state={}; desired={}; calls=0; writes=0; mode=nil; saving=false
            for i=1,19 do state[i]=i==4; desired[i]=i==19 end
            C_EquipmentSet={
                IsSlotIgnoredForSave=function(i)
                    if mode=='read_error' and saving and i==19 then error('restricted') end
                    return state[i]
                end,
                IgnoreSlotForSave=function(i)
                    writes=writes+1
                    if mode=='restore_error' and saving and i==4 then error('restore failed') end
                    state[i]=true
                    if mode=='setup_after_effect' and i==19 then error('after effect') end
                end,
                UnignoreSlotForSave=function(i) writes=writes+1; state[i]=false end,
            }
        end
        function save()
            calls=calls+1; saving=true
            assert(state[4]==false and state[19]==true)
            if mode=='throw' then error('native failure after effect') end
            if mode=='bystander' then state[3]=true end
            if mode=='nested' then assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save)) end
            if mode=='mutate_request' then desired[4]=true end
            return false
        end
        reset()
    "#,
    )
    .exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentSetSaveState.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        assert(AO.WithEquipmentSetSaveSlots(desired,save))
        assert(calls==1 and writes==4 and state[4] and not state[19])
        for _,value in ipairs({'nested','mutate_request'}) do
            reset(); mode=value; assert(AO.WithEquipmentSetSaveSlots(desired,save))
            assert(calls==1 and state[4] and not state[19])
        end
        reset(); mode='throw'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==1 and state[4] and not state[19])
        reset(); mode='setup_after_effect'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==0 and state[4] and not state[19])
        reset(); mode='bystander'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==1 and state[3] and state[4] and not state[19]) -- preserve unrelated edit
        reset(); mode='restore_error'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==1 and not state[4] and not state[19]) -- other cleanup still attempted
        reset(); mode='read_error'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==1 and state[4] and state[19]) -- no blind cleanup of unreadable state
        reset(); desired[1]=nil; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==0 and writes==0)
        reset(); desired[20]=false; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==0 and writes==0)
        reset(); state[1]=nil; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==0 and writes==0)
        reset(); AO.family='classic'; assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save))
        assert(calls==0 and writes==0); AO.family='retail'
        reset(); C_EquipmentSet.IgnoreSlotForSave=nil
        assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save) and calls==0 and writes==0)
        reset(); local reads=0
        C_EquipmentSet.IsSlotIgnoredForSave=function(i)
            reads=reads+1; if reads==20 then state[1]=true end; return state[i]
        end
        assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save) and calls==0 and writes==0)
        reset(); function issecretvalue(v) return v==true end
        assert(not pcall(AO.WithEquipmentSetSaveSlots,desired,save) and calls==0 and writes==0)
        issecretvalue=nil
        reset(); for i=1,19 do desired[i]=state[i] end
        assert(AO.WithEquipmentSetSaveSlots(desired,function() calls=calls+1 end))
        assert(calls==1 and writes==0)
    "#).exec()
}
