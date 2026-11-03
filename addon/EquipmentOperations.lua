-- One exact bag-to-equipment swap, invoked only by a player Apply handler.
local _, AO = ...
local function integer(n,lo,hi)
    return type(n)=="number" and n%1==0 and n>=lo and n<=hi
end
function AO.ApplyEquipmentOperation(op)
    assert(AO.family=="retail" and not InCombatLockdown(), "Equipment changes unavailable")
    local _,_,_,interface=GetBuildInfo()
    assert(interface==AO.interface, "Client interface changed")
    assert(type(op)=="table" and op.kind=="equip_bag_item", "Invalid equipment operation")
    assert(integer(NUM_TOTAL_EQUIPPED_BAG_SLOTS,0,64) and integer(op.bag,0,NUM_TOTAL_EQUIPPED_BAG_SLOTS), "Invalid carried bag")
    assert(integer(op.slot,1,C_Container.GetContainerNumSlots(op.bag)) and integer(op.target_slot,1,19), "Invalid slot")
    assert(type(op.source_guid)=="string" and #op.source_guid>0 and type(op.target_guid)=="string", "Item identities required")
    assert(GetCursorInfo()==nil, "Cursor already occupied")
    assert(not IsInventoryItemLocked(op.target_slot), "Equipment slot locked")
    assert(not UnitCastingInfo("player") and not UnitChannelInfo("player"), "Player casting")
    local source=ItemLocation:CreateFromBagAndSlot(op.bag,op.slot)
    local target=ItemLocation:CreateFromEquipmentSlot(op.target_slot)
    assert(C_Item.GetItemGUID(source)==op.source_guid, "Source item changed")
    assert((C_Item.GetItemGUID(target) or "")==op.target_guid, "Target item changed")
    local info=C_Container.GetContainerItemInfo(op.bag,op.slot)
    assert(info and not info.isLocked and info.stackCount==1 and info.itemID==op.item_id, "Source item unavailable")
    local _,_,_,equipType=C_Item.GetItemInfoInstant(info.hyperlink)
    assert(equipType and equipType~="" and equipType~="INVTYPE_2HWEAPON", "Item needs unsupported multi-slot handling")
    local previousLink=GetInventoryItemLink("player",op.target_slot)
    local previousID=GetInventoryItemID("player",op.target_slot)
    -- The caller checkpoints outcome_unknown before entry. Do not clear another
    -- cursor or automatically retry/undo after any call below.
    C_Container.PickupContainerItem(op.bag,op.slot)
    local cursorKind,cursorID,cursorLink=GetCursorInfo()
    assert(cursorKind=="item" and cursorID==op.item_id and cursorLink==info.hyperlink, "Source pickup unconfirmed")
    assert(C_PaperDollInfo.CanCursorCanGoInSlot(op.target_slot), "Item incompatible with target; cursor retained")
    PickupInventoryItem(op.target_slot)
    assert(C_Item.GetItemGUID(target)==op.source_guid, "Equipment change unconfirmed")
    if GetCursorInfo()~=nil then
        cursorKind,cursorID,cursorLink=GetCursorInfo()
        assert(op.target_guid~="" and cursorKind=="item" and cursorID==previousID and cursorLink==previousLink, "Unexpected cursor after equip")
        assert(not C_Item.GetItemGUID(source), "Source slot occupied; cursor retained")
        C_Container.PickupContainerItem(op.bag,op.slot)
    end
    assert(GetCursorInfo()==nil and (C_Item.GetItemGUID(source) or "")==op.target_guid, "Displaced item return unconfirmed")
    return {status="applied",kind=op.kind}
end
