use mlua::Lua;
#[test]
fn action_moves_swaps_and_refusals_preserve_cursor() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(
        r#"
 AO={family='retail',variant='live',interface=120105,collectors={}}
 function GetBuildInfo() return '', '70688', '',120105 end
 function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
 function InCombatLockdown() return combat end
 function UnitCastingInfo() end; function UnitChannelInfo() end
 function GetActionInfo(slot) local a=slots[slot]; if a then return a.kind,a.id,a.sub_type end end
 function GetMacroInfo() return 'Macro',134400,macroBody end
 function GetCursorInfo() if cursor then return cursor.kind,cursor.id,nil,cursor.id end end
 function AO.collectors.action_bar_observation() return {slots=coverage} end
 function reset()
  slots={[1]={kind='spell',id=123},[2]={kind='macro',id=121}}
  cursor=nil; writes=0; combat=false; reject=false; macroBody='/cast Test'
  coverage={['1']={},['2']={}}
 end
 function PickupAction(slot,copy)
  assert(copy==false); writes=writes+1; cursor=slots[slot]; slots[slot]=nil
 end
 function PlaceAction(slot)
  writes=writes+1; if reject then return end
  slots[slot],cursor=cursor,slots[slot]
 end
 reset()
 op={kind='move_action_slot',family='retail',variant='live',build='70688',interface=120105,
 character='Test',realm='Test',source_slot=1,target_slot=2,
 source={empty=false,kind='spell',id=123},
 target={empty=false,kind='macro',id=121,macro={name='Macro',icon=134400,body='/cast Test'}}}
 "#,
    )
    .exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/ActionOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
 combat=true; assert(not pcall(AO.ApplyActionOperation,op)); combat=false
 macroBody='changed'; assert(not pcall(AO.ApplyActionOperation,op)); macroBody='/cast Test'
 coverage['2']=nil; assert(not pcall(AO.ApplyActionOperation,op)); coverage['2']={}
 cursor={kind='item',id=4}; assert(not pcall(AO.ApplyActionOperation,op)); cursor=nil
 local place=PlaceAction; PlaceAction=nil
 assert(not pcall(AO.ApplyActionOperation,op) and writes==0); PlaceAction=place
 assert(writes==0)
 assert(AO.ApplyActionOperation(op).status=='applied' and writes==3)
 assert(slots[1].kind=='macro' and slots[2].id==123 and cursor==nil)
 assert(not pcall(AO.ApplyActionOperation,op) and writes==3)
 reset(); slots[2]=nil; op.target={empty=true}
 assert(AO.ApplyActionOperation(op).status=='applied' and writes==2 and not slots[1])
 reset(); slots[2]=nil; reject=true
 assert(AO.ApplyActionOperation(op).status=='outcome_unknown' and writes==2 and cursor.id==123)
 reset(); slots[2]=nil
 issecretvalue=function(v) return v==123 end
 assert(not pcall(AO.ApplyActionOperation,op) and writes==0); issecretvalue=nil
 reset(); slots[2]=nil
 local cursorInfo=GetCursorInfo
 GetCursorInfo=function() if cursor then return 'spell',123,nil,nil end end
 assert(not pcall(AO.ApplyActionOperation,op) and writes==1 and cursor.id==123 and slots[2]==nil)
 GetCursorInfo=cursorInfo
 op.kind='copy_action_slot'
 local function pickup(kind,id)
  writes=writes+1; cursor={kind=kind,id=id}
  if changeSource then slots[1].id=999 end
  if wrongCursor then cursor.id=999 end
 end
 C_Spell={PickupSpell=function(id) pickup('spell',id) end}
 C_Item={PickupItem=function(id) pickup('item',id) end}
 PickupMacro=function(id) pickup('macro',id) end
 for _,kind in ipairs({'spell','item','macro'}) do
  reset(); slots[2]=nil; slots[1]={kind=kind,id=kind=='macro' and 121 or 123}
  op.source={empty=false,kind=kind,id=slots[1].id}
  if kind=='macro' then op.source.macro={name='Macro',icon=134400,body='/cast Test'} end
  assert(AO.ApplyActionOperation(op).status=='applied' and writes==2)
  assert(slots[1].id==slots[2].id and slots[1].kind==kind and cursor==nil)
  assert(not pcall(AO.ApplyActionOperation,op) and writes==2)
 end
 op.source={empty=false,kind='spell',id=123}
 reset(); op.target={empty=false,kind='macro',id=121,macro={name='Macro',icon=134400,body='/cast Test'}}
 assert(not pcall(AO.ApplyActionOperation,op) and writes==0)
 op.target={empty=true}; reset(); slots[2]=nil; reject=true
 assert(AO.ApplyActionOperation(op).status=='outcome_unknown' and writes==2 and slots[1].id==123 and cursor.id==123)
 reject=false; slots[2]=cursor; cursor=nil
 assert(AO.VerifyActionOperation(op) and writes==2)
 reset(); slots[2]=nil; changeSource=true
 assert(not pcall(AO.ApplyActionOperation,op) and writes==1 and not slots[2]); changeSource=false
 reset(); slots[2]=nil; wrongCursor=true
 assert(not pcall(AO.ApplyActionOperation,op) and writes==1 and cursor.id==999); wrongCursor=false
 reset(); slots[2]=nil; C_Spell.PickupSpell=nil
 assert(not pcall(AO.ApplyActionOperation,op) and writes==0)
 "#,
    )
    .exec()
}
