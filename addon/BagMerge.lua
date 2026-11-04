-- Player-applied full-stack merge. No retry, cursor clearing or item use.
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
local function empty(entry)
    return not C_Container.GetContainerItemInfo(entry.bag,entry.slot)
        and not C_Item.GetItemGUID(ItemLocation:CreateFromBagAndSlot(entry.bag,entry.slot))
end
local function cursorMatches(entry)
    local kind,id,link=GetCursorInfo()
    return kind=="item" and id==entry.item_id and link==entry.link
end
function AO.VerifyBagMerge(op)
    local target=op.target
    local combined={bag=target.bag,slot=target.slot,item_id=target.item_id,
        count=op.source.count+target.count,link=target.link,guid=target.guid}
    return not not (sameClient(op) and GetCursorInfo()==nil and empty(op.source) and matches(combined))
end
function AO.ApplyBagMerge(op)
    assert(type(op)=="table" and op.kind=="merge_bag_items" and sameClient(op),"Bag merge client mismatch")
    assert(not InCombatLockdown() and not UnitCastingInfo("player") and not UnitChannelInfo("player"),"Bag merge unavailable")
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
    assert(source.bag~=target.bag or source.slot~=target.slot,"Merge locations must differ")
    assert(source.guid~=target.guid,"Merge identities must differ")
    assert(source.item_id==target.item_id and source.link==target.link,"Merge item descriptions differ")
    assert(matches(source) and matches(target),"Bag merge preimage changed or locked")
    assert(type(C_Item.GetItemMaxStackSize)=="function","Stack capacity API unavailable")
    local function capacity(entry)
        return C_Item.GetItemMaxStackSize(ItemLocation:CreateFromBagAndSlot(entry.bag,entry.slot))
    end
    local maximum=capacity(source)
    assert(integer(maximum,1,2147483647) and capacity(target)==maximum,"Stack capacity unavailable or inconsistent")
    assert(source.count+target.count<=maximum,"Complete source stack does not fit")
    assert(type(AO.CheckBagCompatibility)=="function"
        and AO.CheckBagCompatibility(source.link,target.bag).status=="compatible","Merge bag compatibility unconfirmed")
    assert(capacity(source)==maximum and capacity(target)==maximum,"Stack capacity changed")
    assert(matches(source) and matches(target),"Bag merge preimage changed during preflight")
    -- Caller checkpoints unknown before entry. Pickup can retain the source observation.
    C_Container.PickupContainerItem(source.bag,source.slot)
    assert(cursorMatches(source) and (empty(source) or matches(source)) and matches(target),"Source pickup unconfirmed; inspect cursor")
    C_Container.PickupContainerItem(target.bag,target.slot)
    -- Hidden native stack restrictions can still reject placement. Never retry.
    return {kind=op.kind,status=AO.VerifyBagMerge(op) and "applied" or "outcome_unknown"}
end
