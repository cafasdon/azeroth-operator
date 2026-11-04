use mlua::Lua;
#[test]
fn split_requires_quantity_conservation_and_distinct_identity() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
 AO={family='retail',interface=120105}; NUM_TOTAL_EQUIPPED_BAG_SLOTS=5; writes=0
 function reset() slots={[1]={guid='original',itemID=123,stackCount=20,hyperlink='link'}}; cursor=nil end
 reset()
 function GetBuildInfo() return '', '', '',120105 end
 function InCombatLockdown() return false end
 function UnitCastingInfo() end; function UnitChannelInfo() end
 function GetCursorInfo() if cursor then return 'item',cursor.itemID,cursor.hyperlink end end
 ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return slot end}
 C_Item={GetItemGUID=function(loc) return slots[loc] and slots[loc].guid end}
 C_Container={GetContainerNumSlots=function() return 10 end,
 GetContainerItemInfo=function(_,slot) local x=slots[slot]; if x then return {guid=x.guid,itemID=x.itemID,stackCount=x.stackCount,hyperlink=x.hyperlink} end end,
 SplitContainerItem=function(_,slot,amount)
  writes=writes+1; slots[slot].stackCount=slots[slot].stackCount-amount
  cursor={guid='new',itemID=123,stackCount=amount,hyperlink='link'}
 end,
 PickupContainerItem=function(_,slot)
  writes=writes+1; if reject then return end
  slots[slot],cursor=cursor,slots[slot]
  if wrongCount then slots[slot].stackCount=1 end
 end}
 "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("bag_metadata_fixture.lua")).exec()?;
    lua.load(include_str!("../addon/BagCompatibility.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/BagOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
 local op={kind='split_bag_item',bag=0,slot=1,target_bag=0,target_slot=2,source_guid='original',item_id=123,count=20,amount=5,expect_empty_target=true}
 for _,amount in ipairs({0,20,21,1.5}) do op.amount=amount; assert(not pcall(AO.ApplyBagOperation,op)) end
 assert(writes==0); op.amount=5
 local classifier=ContainerFrame_IsReagentBag
 ContainerFrame_IsReagentBag=function() return true end
 assert(not pcall(AO.ApplyBagOperation,op) and writes==0)
 ContainerFrame_IsReagentBag=classifier
 local family=C_Item.GetItemFamily;C_Item.GetItemFamily=function() end
 assert(not pcall(AO.ApplyBagOperation,op) and writes==0);C_Item.GetItemFamily=family
 assert(AO.ApplyBagOperation(op).status=='applied')
 assert(writes==2 and slots[1].stackCount==15 and slots[2].stackCount==5 and cursor==nil)
 reset(); reject=true
 assert(AO.ApplyBagOperation(op).status=='outcome_unknown')
 assert(writes==4 and slots[1].stackCount==15 and cursor.stackCount==5)
 reset(); reject=false; wrongCount=true
 assert(AO.ApplyBagOperation(op).status=='outcome_unknown' and writes==6)
 reset(); wrongCount=false
 C_Container.SplitContainerItem=function(_,slot,amount)
  writes=writes+1; cursor={guid='new',itemID=123,stackCount=amount,hyperlink='link'}
 end
 assert(AO.ApplyBagOperation(op).status=='outcome_unknown' and writes==8)
 assert(slots[1].stackCount==20 and slots[2].stackCount==5)
 assert(not AO.VerifyBagOperation(op))
 slots[1].stackCount=15
 assert(AO.VerifyBagOperation(op) and writes==8)
 slots[2].guid='original'; assert(not AO.VerifyBagOperation(op))
 "#).exec()
}
