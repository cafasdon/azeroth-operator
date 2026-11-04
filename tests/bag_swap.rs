use mlua::Lua;
#[test]
fn swap_checks_both_identities_and_never_retries_uncertainty() -> mlua::Result<()> {
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
   slots={['0:1']={itemID=123,stackCount=2,hyperlink='source',guid='one'},['1:2']={itemID=456,stackCount=3,hyperlink='target',guid='two'}}
   op={kind='swap_bag_items',family='retail',variant='live',build='70688',interface=120105,character='Test',realm='Test',
   source={bag=0,slot=1,item_id=123,count=2,link='source',guid='one'},target={bag=1,slot=2,item_id=456,count=3,link='target',guid='two'}}
   C_Container.PickupContainerItem=function(bag,slot)
     writes=writes+1;local key=bag..':'..slot
     if reject==writes then return end
     slots[key],cursor=cursor,slots[key]
     if lag and writes==2 then slots['0:1']={itemID=123,stackCount=2,hyperlink='source',guid='one'} end
     if race and writes==1 then slots['1:2'].guid='intruder' end
   end
 end
 "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/BagSwap.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
 reset(); assert(AO.ApplyBagSwap(op).status=='applied' and writes==3 and not cursor)
 assert(slots['0:1'].guid=='two' and slots['1:2'].guid=='one')
 assert(not pcall(AO.ApplyBagSwap,op) and writes==3)
 reset();combat=true;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();op.target.guid='wrong';assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();slots['1:2'].isLocked=true;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();op.target.item_id=123;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();op.target.bag=6;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();op.source.count=1;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 reset();race=true;assert(not pcall(AO.ApplyBagSwap,op) and writes==1 and cursor.guid=='one')
 reset();reject=2;assert(AO.ApplyBagSwap(op).status=='outcome_unknown' and writes==2 and cursor.guid=='one')
 reset();reject=3;assert(AO.ApplyBagSwap(op).status=='outcome_unknown' and writes==3 and cursor.guid=='two')
 assert(not AO.VerifyBagSwap(op) and writes==3)
 slots['0:1'],cursor=cursor,nil;assert(AO.VerifyBagSwap(op) and writes==3)
 reset();lag=true;assert(AO.ApplyBagSwap(op).status=='outcome_unknown' and writes==2 and cursor.guid=='two')
 reset();C_Container.PickupContainerItem=nil;assert(not pcall(AO.ApplyBagSwap,op) and writes==0)
 "#).exec()
}
