use mlua::Lua;
#[test]
fn merge_conserves_quantity_and_refuses_capacity_uncertainty() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
 AO={family='retail',variant='live',interface=120105};NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
 function GetBuildInfo() return '', '70688','',120105 end
 function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
 function InCombatLockdown() return combat end
 function UnitCastingInfo() end;function UnitChannelInfo() end
 ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return bag..':'..slot end}
 function GetCursorInfo() if cursor then return 'item',cursor.itemID,cursor.hyperlink end end
 C_Item={GetItemGUID=function(loc) return slots[loc] and slots[loc].guid end}
 C_Container={GetContainerNumSlots=function() return 10 end,
 GetContainerItemInfo=function(bag,slot) return slots[bag..':'..slot] end}
 function reset()
   writes=0;cursor=nil;combat=false;reject=nil;lag=nil;race=nil
   slots={['0:1']={itemID=123,stackCount=2,hyperlink='source',guid='one'},['1:2']={itemID=123,stackCount=3,hyperlink='source',guid='two'}}
   op={kind='merge_bag_items',family='retail',variant='live',build='70688',interface=120105,character='Test',realm='Test',
   source={bag=0,slot=1,item_id=123,count=2,link='source',guid='one'},target={bag=1,slot=2,item_id=123,count=3,link='source',guid='two'}}
   C_Container.PickupContainerItem=function(bag,slot)
     writes=writes+1;local key=bag..':'..slot
     if reject==writes then return end
     if writes==1 then cursor=slots[key];slots[key]=nil
     else slots[key].stackCount=slots[key].stackCount+cursor.stackCount;cursor=nil end
     if lag and writes==2 then slots['0:1']={itemID=123,stackCount=2,hyperlink='source',guid='one'} end
     if race and writes==1 then slots['1:2'].guid='intruder' end
   end
 end
 "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("bag_metadata_fixture.lua")).exec()?;
    lua.load(include_str!("../addon/BagCompatibility.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/BagMerge.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
 C_Item.GetItemMaxStackSize=function() return maximum end
 reset();maximum=20;assert(AO.ApplyBagMerge(op).status=='applied' and writes==2 and not cursor)
 assert(not slots['0:1'] and slots['1:2'].stackCount==5 and slots['1:2'].guid=='two')
 assert(not pcall(AO.ApplyBagMerge,op) and writes==2)
 reset();maximum=4;assert(not pcall(AO.ApplyBagMerge,op) and writes==0)
 reset();maximum=nil;assert(not pcall(AO.ApplyBagMerge,op) and writes==0)
 reset();maximum=20;slots['1:2'].isLocked=true;assert(not pcall(AO.ApplyBagMerge,op) and writes==0)
 reset();op.target.link='different';assert(not pcall(AO.ApplyBagMerge,op) and writes==0)
 reset();reject=2;assert(AO.ApplyBagMerge(op).status=='outcome_unknown' and writes==2 and cursor)
 assert(not AO.VerifyBagMerge(op) and writes==2)
 slots['1:2'].stackCount=5;cursor=nil;assert(AO.VerifyBagMerge(op) and writes==2)
 reset();local reads=0;C_Item.GetItemMaxStackSize=function() reads=reads+1;return reads<3 and 20 or 10 end
 assert(not pcall(AO.ApplyBagMerge,op) and writes==0)
 "#).exec()
}
