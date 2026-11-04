use mlua::Lua;
#[test]
fn retained_source_and_delayed_readback_do_not_repeat_writes() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(
        r#"
 AO={family='retail',variant='live',interface=120105}; NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
 function GetBuildInfo() return '', '70688', '',120105 end
 function InCombatLockdown() return false end
 function UnitCastingInfo() end; function UnitChannelInfo() end
 function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
 function GetTime() return now end
 function GetCursorInfo() if cursor then return 'item',123,'link' end end
 ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return slot end}
 C_Item={GetItemGUID=function(loc) return slots[loc] and slots[loc].guid end}
 C_Container={GetContainerNumSlots=function() return 10 end,
 GetContainerItemInfo=function(_,slot) return slots[slot] end,
 PickupContainerItem=function(_,slot)
  writes=writes+1
  if slot==1 then cursor=true; slots[1].isLocked=true
  else cursor=nil; placed=true end
 end}
 function reset()
  slots={[1]={guid='original',itemID=123,stackCount=20,hyperlink='link'}}
  cursor=nil; placed=false; writes=0; now=0
 end
 function settle()
  slots[2]=slots[1]; slots[1]=nil; slots[2].isLocked=false
 end
 reset()
 op={id=string.rep('a',64),family='retail',variant='live',build='70688',interface=120105,
 character='Test',realm='Test',kind='move_bag_item',bag=0,slot=1,target_bag=0,target_slot=2,
 source_guid='original',item_id=123,count=20,expect_empty_target=true}
 function AO.DecodeBagOperation() return op end
 "#,
    )
    .exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("bag_metadata_fixture.lua")).exec()?;
    lua.load(include_str!("../addon/BagCompatibility.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    for source in [
        include_str!("../addon/BagOperations.lua"),
        include_str!("../addon/MacroImport.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(
        r#"
 local receipt,ok=AO.ApplyImportedMacro('AO-BAG-1:fixture')
 assert(ok and receipt.status=='outcome_unknown' and writes==2 and placed)
 assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==2)
 settle(); slots[2].guid='different'
 assert(not AO.VerifyBagOperation(op))
 slots[2].guid='original'; slots[2].isLocked=true
 assert(not AO.VerifyBagOperation(op))
 slots[2].isLocked=false; now=10
 assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==2)
 assert(not pcall(AO.ApplyImportedMacro,'AO-BAG-1:fixture') and writes==2)
 reset(); op.id=string.rep('b',64)
 assert(AO.ApplyImportedMacro('AO-BAG-1:fixture').status=='outcome_unknown')
 settle(); now=61
 assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==2)
 reset(); op.id=string.rep('c',64)
 AO.ApplyImportedMacro('AO-BAG-1:fixture'); settle()
 function UnitName() return 'Other' end
 assert(not AO.VerifyBagOperation(op))
 assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==2)
 "#,
    )
    .exec()
}
