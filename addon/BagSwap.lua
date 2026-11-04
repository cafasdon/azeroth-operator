-- Player-applied full-stack swap. No retry, cursor clearing or item use.
local _, AO = ...
local function integer(n,lo,hi) return type(n)=="number" and n%1==0 and n>=lo and n<=hi end
local function sameClient(op)
    local _,build,_,interface=GetBuildInfo()
    return AO.family=="retail" and op.family==AO.family and op.variant==AO.variant
        and op.build==tostring(build) and interface==AO.interface and op.interface==interface
        and op.character==UnitName("player") and op.realm==GetRealmName()
end
local function matches(entry)
    local info=C_Container.GetContainerItemInfo(entry.bag,entry.slot)
    local location=ItemLocation:CreateFromBagAndSlot(entry.bag,entry.slot)
    return info and not info.isLocked and info.itemID==entry.item_id and info.stackCount==entry.count
        and info.hyperlink==entry.link and C_Item.GetItemGUID(location)==entry.guid
end
local function at(entry,location)
    return {bag=location.bag,slot=location.slot,item_id=entry.item_id,count=entry.count,link=entry.link,guid=entry.guid}
end
local function empty(entry)
    return not C_Container.GetContainerItemInfo(entry.bag,entry.slot)
        and not C_Item.GetItemGUID(ItemLocation:CreateFromBagAndSlot(entry.bag,entry.slot))
end
local function cursorMatches(entry)
    local kind,id,link=GetCursorInfo()
    return kind=="item" and id==entry.item_id and link==entry.link
end
function AO.VerifyBagSwap(op)
    return not not (sameClient(op) and GetCursorInfo()==nil
        and matches(at(op.source,op.target)) and matches(at(op.target,op.source)))
end
function AO.ApplyBagSwap(op)
    assert(type(op)=="table" and op.kind=="swap_bag_items" and sameClient(op),"Bag swap client mismatch")
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"),"Bag swap unavailable")
    assert(GetCursorInfo()==nil,"Cursor already occupied")
    assert(type(C_Container.PickupContainerItem)=="function","Bag pickup API unavailable")
    assert(integer(NUM_TOTAL_EQUIPPED_BAG_SLOTS,0,64),"Carried bag boundary unavailable")
    for _,entry in ipairs({op.source,op.target}) do
        assert(type(entry)=="table" and integer(entry.bag,0,NUM_TOTAL_EQUIPPED_BAG_SLOTS),"Invalid carried bag")
        assert(integer(entry.slot,1,C_Container.GetContainerNumSlots(entry.bag)),"Invalid carried slot")
        assert(integer(entry.item_id,1,4294967295) and integer(entry.count,1,2147483647),"Invalid item preimage")
        assert(type(entry.guid)=="string" and #entry.guid>0 and type(entry.link)=="string" and #entry.link>0,"Exact item identity required")
    end
    local source,target=op.source,op.target
    assert(source.bag~=target.bag or source.slot~=target.slot,"Swap locations must differ")
    assert(source.guid~=target.guid,"Swap identities must differ")
    -- Same item IDs may merge rather than swap; never infer stack compatibility.
    assert(source.item_id~=target.item_id,"Same-item swaps require stack compatibility support")
    assert(matches(source) and matches(target),"Bag swap preimage changed or locked")
    -- Caller checkpoints unknown before entry. Pickup can retain the source observation.
    C_Container.PickupContainerItem(source.bag,source.slot)
    assert(cursorMatches(source) and (empty(source) or matches(source)) and matches(target),"Source pickup unconfirmed; inspect cursor")
    C_Container.PickupContainerItem(target.bag,target.slot)
    -- Never place a displaced item over a stale/occupied source. If observations lag,
    -- stop with the cursor retained; an observer can verify but cannot finish writes.
    if not (cursorMatches(target) and empty(source) and matches(at(source,target))) then
        return {kind=op.kind,status="outcome_unknown"}
    end
    C_Container.PickupContainerItem(source.bag,source.slot)
    return {kind=op.kind,status=AO.VerifyBagSwap(op) and "applied" or "outcome_unknown"}
end
