-- Read-only eligibility preflight; not a guarantee of native placement success.
local _,AO=...
local function integer(n,lo,hi) return type(n)=='number' and n%1==0 and n>=lo and n<=hi end
local function observe(link,bag)
    assert(AO.family=='retail' and not InCombatLockdown(),'Bag compatibility unavailable')
    local _,_,_,interface=GetBuildInfo();assert(interface==AO.interface,'Client interface changed')
    assert(type(link)=='string' and #link>0 and #link<=4096,'Item link required')
    assert(integer(NUM_TOTAL_EQUIPPED_BAG_SLOTS,0,64) and integer(bag,0,NUM_TOTAL_EQUIPPED_BAG_SLOTS),'Invalid carried bag')
    assert(C_Container and type(C_Container.GetContainerNumSlots)=='function'
        and type(C_Container.GetContainerNumFreeSlots)=='function','Container metadata unavailable')
    assert(C_Item and type(C_Item.GetItemFamily)=='function' and type(C_Item.GetItemInfo)=='function','Item metadata unavailable')
    assert(type(ContainerFrame_IsReagentBag)=='function','Reagent bag classifier unavailable')
    local slots=C_Container.GetContainerNumSlots(bag)
    local free,family=C_Container.GetContainerNumFreeSlots(bag)
    assert(integer(slots,1,1024) and integer(free,0,slots) and integer(family,0,4294967295),'Bag metadata unavailable')
    local itemFamily=C_Item.GetItemFamily(link)
    local name,_,_,_,_,_,_,_,equipLoc,_,_,_,_,_,_,_,reagent=C_Item.GetItemInfo(link)
    local reagentBag=ContainerFrame_IsReagentBag(bag)
    assert(type(name)=='string' and #name>0 and type(equipLoc)=='string'
        and type(reagent)=='boolean' and type(reagentBag)=='boolean'
        and integer(itemFamily,0,4294967295),'Item metadata unavailable')
    return {slots=slots,free=free,family=family,item_family=itemFamily,equip_loc=equipLoc,
        reagent=reagent,reagent_bag=reagentBag}
end
local function same(a,b)
    for k,v in pairs(a) do if b[k]~=v then return false end end
    return true
end
function AO.CheckBagCompatibility(link,bag)
    local ok,result=pcall(function()
        local first=observe(link,bag);local second=observe(link,bag)
        assert(same(first,second),'Bag compatibility metadata changed')
        local accepted,reason
        if first.reagent_bag then accepted=first.reagent;reason='reagent_bag'
        elseif first.family==0 then accepted=true;reason='general_bag'
        elseif first.equip_loc=='INVTYPE_BAG' then accepted=false;reason='container_item_in_specialised_bag'
        else
            assert(bit and type(bit.band)=='function','Family mask comparison unavailable')
            accepted=bit.band(first.family,first.item_family)~=0;reason='specialised_family'
        end
        return {status=accepted and 'compatible' or 'incompatible',reason=reason,
            observation=first,game_execution=false,live_qualified=false}
    end)
    if ok then return result end
    return {status='unknown',reason='metadata_unavailable_or_changed',game_execution=false,live_qualified=false}
end
