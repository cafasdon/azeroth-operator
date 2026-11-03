use mlua::Lua;
#[test]
fn exact_equipment_swap_and_precondition_refusals() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',interface=120105}; writes=0; combat=false; NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        bag={guid='new',id=1,link='newlink'}; gear={guid='old',id=2,link='oldlink'}; cursor=nil
        function InCombatLockdown() return combat end
        function GetBuildInfo() return '', '', '',120105 end
        function UnitCastingInfo() end; function UnitChannelInfo() end
        function IsInventoryItemLocked() return false end
        function GetCursorInfo() if cursor then return 'item',cursor.id,cursor.link end end
        function GetInventoryItemID() return gear and gear.id end
        function GetInventoryItemLink() return gear and gear.link end
        function PickupInventoryItem() writes=writes+1; gear,cursor=cursor,gear end
        ItemLocation={CreateFromBagAndSlot=function() return 'bag' end,CreateFromEquipmentSlot=function() return 'gear' end}
        C_Item={GetItemGUID=function(loc) local i=loc=='bag' and bag or (loc=='gear' and gear); return i and i.guid end,
            GetItemInfoInstant=function() return 1,'Armor','Plate','INVTYPE_HEAD' end}
        C_Container={GetContainerNumSlots=function() return 20 end,
            GetContainerItemInfo=function() if bag then return {itemID=bag.id,hyperlink=bag.link,stackCount=1,isLocked=false} end end,
            PickupContainerItem=function() writes=writes+1; bag,cursor=cursor,bag end}
        C_PaperDollInfo={CanCursorCanGoInSlot=function() return true end}
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        local op={kind='equip_bag_item',bag=0,slot=1,target_slot=1,source_guid='new',target_guid='old',item_id=1}
        combat=true; assert(not pcall(AO.ApplyEquipmentOperation,op)); combat=false
        op.source_guid='stale'; assert(not pcall(AO.ApplyEquipmentOperation,op)); op.source_guid='new'
        op.target_guid='stale'; assert(not pcall(AO.ApplyEquipmentOperation,op)); op.target_guid='old'
        cursor={guid='unrelated'}; assert(not pcall(AO.ApplyEquipmentOperation,op)); cursor=nil
        assert(writes==0)
        assert(AO.ApplyEquipmentOperation(op).status=='applied')
        assert(writes==3 and gear.guid=='new' and bag.guid=='old' and cursor==nil)
        assert(not pcall(AO.ApplyEquipmentOperation,op) and writes==3)
        bag={guid='new',id=1,link='newlink'}; gear={guid='old',id=2,link='oldlink'}
        C_Item.GetItemInfoInstant=function() return 1,'Weapon','Sword','INVTYPE_2HWEAPON' end
        assert(not pcall(AO.ApplyEquipmentOperation,op) and writes==3)
        C_Item.GetItemInfoInstant=function() return 1,'Armor','Plate','INVTYPE_HEAD' end
        C_PaperDollInfo.CanCursorCanGoInSlot=function() return false end
        assert(not pcall(AO.ApplyEquipmentOperation,op))
        assert(writes==4 and cursor.guid=='new' and gear.guid=='old')
    "#).exec()
}
