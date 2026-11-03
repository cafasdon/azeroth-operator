-- One full-stack move or split to an explicitly empty carried slot, from a player Apply.
local _, AO = ...
local function integer(n,lo,hi)
    return type(n)=="number" and n%1==0 and n>=lo and n<=hi
end
function AO.ApplyBagOperation(op)
    assert(AO.family=="retail" and not InCombatLockdown(), "Bag changes unavailable")
    local _,_,_,interface=GetBuildInfo()
    assert(interface==AO.interface, "Client interface changed")
    assert(type(op)=="table" and (op.kind=="move_bag_item" or op.kind=="split_bag_item"), "Invalid bag operation")
    assert(integer(NUM_TOTAL_EQUIPPED_BAG_SLOTS,0,64), "Carried bag boundary unavailable")
    for _,loc in ipairs({{op.bag,op.slot},{op.target_bag,op.target_slot}}) do
        assert(integer(loc[1],0,NUM_TOTAL_EQUIPPED_BAG_SLOTS), "Invalid carried bag")
        assert(integer(loc[2],1,C_Container.GetContainerNumSlots(loc[1])), "Invalid bag slot")
    end
    assert(op.bag~=op.target_bag or op.slot~=op.target_slot, "Source equals destination")
    assert(type(op.source_guid)=="string" and #op.source_guid>0 and op.expect_empty_target==true, "Identity and empty destination preconditions required")
    assert(integer(op.count,1,2147483647) and integer(op.item_id,1,4294967295), "Invalid item preconditions")
    local splitting=op.kind=="split_bag_item"
    if splitting then assert(integer(op.amount,1,op.count-1), "Split amount must leave a source remainder") end
    assert(GetCursorInfo()==nil, "Cursor already occupied")
    assert(not UnitCastingInfo("player") and not UnitChannelInfo("player"), "Player casting")
    local source=ItemLocation:CreateFromBagAndSlot(op.bag,op.slot)
    local target=ItemLocation:CreateFromBagAndSlot(op.target_bag,op.target_slot)
    local function emptyTarget()
        return not C_Container.GetContainerItemInfo(op.target_bag,op.target_slot) and not C_Item.GetItemGUID(target)
    end
    local info=C_Container.GetContainerItemInfo(op.bag,op.slot)
    assert(info and not info.isLocked and info.itemID==op.item_id and info.stackCount==op.count, "Source changed or locked")
    assert(C_Item.GetItemGUID(source)==op.source_guid and emptyTarget(), "Item locations changed")
    local observedLink=info.hyperlink
    -- Caller must checkpoint uncertainty before entry. Never clear or retry a cursor.
    local function sourceAfterPickup()
        if not splitting then return not C_Item.GetItemGUID(source) end
        local remaining=C_Container.GetContainerItemInfo(op.bag,op.slot)
        return C_Item.GetItemGUID(source)==op.source_guid and remaining and remaining.itemID==op.item_id
            and remaining.hyperlink==observedLink and remaining.stackCount==op.count-op.amount
    end
    -- Pickup may leave the original slot observable until placement/server update.
    -- Accept only the original exact preimage or the expected post-pickup state.
    local function sourceDuringPickup()
        if sourceAfterPickup() then return true end
        local retained=C_Container.GetContainerItemInfo(op.bag,op.slot)
        return C_Item.GetItemGUID(source)==op.source_guid and retained
            and retained.itemID==op.item_id and retained.stackCount==op.count
            and retained.hyperlink==observedLink
    end
    if splitting then C_Container.SplitContainerItem(op.bag,op.slot,op.amount)
    else C_Container.PickupContainerItem(op.bag,op.slot) end
    local kind,id,link=GetCursorInfo()
    assert(kind=="item" and id==op.item_id and link==observedLink, "Source pickup unconfirmed")
    assert(sourceDuringPickup() and emptyTarget(), "Locations changed after pickup; cursor retained")
    C_Container.PickupContainerItem(op.target_bag,op.target_slot)
    -- Keep the link observed before the write, never a later mutable slot table.
    op.observed_link=observedLink
    return {status=AO.VerifyBagOperation(op) and "applied" or "outcome_unknown",kind=op.kind}
end

-- Observation only. Called immediately and, for a bounded window, during export.
function AO.VerifyBagOperation(op)
    if GetCursorInfo()~=nil or AO.family~="retail" then return false end
    local _,build,_,interface=GetBuildInfo()
    if interface~=AO.interface then return false end
    if op.build and (tostring(build)~=op.build or op.variant~=AO.variant
        or op.character~=UnitName("player") or op.realm~=GetRealmName()) then return false end
    if type(op.observed_link)~="string" then return false end
    local source=ItemLocation:CreateFromBagAndSlot(op.bag,op.slot)
    local target=ItemLocation:CreateFromBagAndSlot(op.target_bag,op.target_slot)
    local remaining=C_Container.GetContainerItemInfo(op.bag,op.slot)
    local moved=C_Container.GetContainerItemInfo(op.target_bag,op.target_slot)
    local targetGUID=C_Item.GetItemGUID(target)
    local splitting=op.kind=="split_bag_item"
    local sourceOK=not remaining and not C_Item.GetItemGUID(source)
    local identityOK=targetGUID==op.source_guid
    if splitting then
        sourceOK=remaining and not remaining.isLocked and C_Item.GetItemGUID(source)==op.source_guid
            and remaining.itemID==op.item_id and remaining.hyperlink==op.observed_link
            and remaining.stackCount==op.count-op.amount
        identityOK=type(targetGUID)=="string" and #targetGUID>0 and targetGUID~=op.source_guid
    end
    return not not (sourceOK and identityOK and moved and not moved.isLocked
        and moved.itemID==op.item_id and moved.hyperlink==op.observed_link
        and moved.stackCount==(splitting and op.amount or op.count))
end
