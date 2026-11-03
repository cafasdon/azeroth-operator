use mlua::Lua;
#[test]
fn full_stack_move_guards_and_partial_effects() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='retail',interface=120105}; NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        slots={[1]={guid='stack',itemID=123,stackCount=20,hyperlink='link'}}; writes=0
        function GetBuildInfo() return '', '', '',120105 end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() end; function UnitChannelInfo() end
        function GetCursorInfo() if cursor then return 'item',cursor.itemID,cursor.hyperlink end end
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return slot end}
        C_Item={GetItemGUID=function(loc) return slots[loc] and slots[loc].guid end}
        C_Container={GetContainerNumSlots=function() return 10 end,
            GetContainerItemInfo=function(_,slot) return slots[slot] end,
            PickupContainerItem=function(_,slot)
                writes=writes+1
                if reject and slot==2 then return end
                slots[slot],cursor=cursor,slots[slot]
                if race and slot==1 then slots[2]={guid='other'} end
            end}
    "#,
    )
    .exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/BagOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
        local op={kind='move_bag_item',bag=0,slot=1,target_bag=0,target_slot=2,
            source_guid='stack',item_id=123,count=20,expect_empty_target=true}
        combat=true; assert(not pcall(AO.ApplyBagOperation,op)); combat=false
        op.count=19; assert(not pcall(AO.ApplyBagOperation,op)); op.count=20
        slots[2]={guid='occupied'}; assert(not pcall(AO.ApplyBagOperation,op)); slots[2]=nil
        op.source_guid='wrong'; assert(not pcall(AO.ApplyBagOperation,op)); op.source_guid='stack'
        assert(writes==0)
        assert(AO.ApplyBagOperation(op).status=='applied')
        assert(writes==2 and slots[1]==nil and slots[2].stackCount==20 and cursor==nil)
        assert(not pcall(AO.ApplyBagOperation,op) and writes==2)
        slots[1],slots[2]=slots[2],nil; reject=true
        assert(AO.ApplyBagOperation(op).status=='outcome_unknown')
        assert(writes==4 and cursor.guid=='stack' and slots[1]==nil and slots[2]==nil)
        slots[1],cursor=cursor,nil; reject=false; race=true
        assert(not pcall(AO.ApplyBagOperation,op))
        assert(writes==5 and cursor.guid=='stack' and slots[2].guid=='other')
    "#,
    )
    .exec()
}
