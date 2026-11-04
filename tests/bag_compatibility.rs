use mlua::Lua;
#[test]
fn compatibility_is_read_only_and_missing_metadata_is_unknown() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
 AO={family='retail',interface=120105};NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
 function GetBuildInfo() return '', '', '',120105 end
 function InCombatLockdown() return false end
 C_Container={GetContainerNumSlots=function() return 10 end,
 GetContainerNumFreeSlots=function() reads=reads+1;return 0,race and reads==2 and 8 or family end}
 C_Item={GetItemFamily=function() return itemFamily end,
 GetItemInfo=function() return 'Item',nil,nil,nil,nil,nil,nil,nil,equipLoc,nil,nil,nil,nil,nil,nil,nil,reagent end}
 function ContainerFrame_IsReagentBag(bag) return bag==5 end
 bit={band=function(a,b)
 local result,p=0,1
 for i=1,32 do if a%2==1 and b%2==1 then result=result+p end;a=math.floor(a/2);b=math.floor(b/2);p=p*2 end
 return result end}
 function reset() reads=0;family=0;itemFamily=0;equipLoc='';reagent=false;race=false end
 "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/BagCompatibility.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
 reset();assert(AO.CheckBagCompatibility('link',0).status=='compatible' and reads==2)
 reset();assert(AO.CheckBagCompatibility('link',5).status=='incompatible')
 reset();reagent=true;assert(AO.CheckBagCompatibility('link',5).status=='compatible')
 reset();family=32;itemFamily=96;assert(AO.CheckBagCompatibility('link',1).status=='compatible')
 reset();family=32;itemFamily=64;assert(AO.CheckBagCompatibility('link',1).status=='incompatible')
 reset();family=32;itemFamily=32;equipLoc='INVTYPE_BAG';assert(AO.CheckBagCompatibility('link',1).status=='incompatible')
 reset();itemFamily=nil;assert(AO.CheckBagCompatibility('link',0).status=='unknown')
 reset();family=nil;assert(AO.CheckBagCompatibility('link',0).status=='unknown')
 reset();reagent=nil;assert(AO.CheckBagCompatibility('link',5).status=='unknown')
 reset();race=true;assert(AO.CheckBagCompatibility('link',1).status=='unknown')
 reset();assert(AO.CheckBagCompatibility('link',6).status=='unknown' and reads==0)
 reset();C_Item.GetItemInfo=nil;assert(AO.CheckBagCompatibility('link',1).status=='unknown')
 "#).exec()
}
