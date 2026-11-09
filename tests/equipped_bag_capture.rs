use mlua::Lua;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};accesses=0;actions=0
        SECRET=setmetatable({}, {__index=function() accesses=accesses+1;error('PRIVATE') end,
            __call=function() accesses=accesses+1;error('PRIVATE') end})
        function issecretvalue(v) return rawequal(v,SECRET) end
        function reset()
            accesses=0;actions=0;itemCalls=0
            C_Container={GetContainerNumSlots=function(bag) return bag==0 and 16 or bag==1 and 8 or bag==2 and 12 or 0 end,
                GetContainerNumFreeSlots=function() return 2,0 end,
                ContainerIDToInventoryID=function(bag) return 19+bag end}
            GetInventoryItemID=function(unit,slot) assert(unit=='player' and slot>=20);itemCalls=itemCalls+1;return slot==20 and 8217 or 100 end
            C_Container.PickupContainerItem=function() actions=actions+1;error('PRIVATE action') end
        end
        reset()
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

#[test]
fn matching_identity_and_optional_failure_preserve_base() -> mlua::Result<()> {
    let lua = fixture()?;
    lua.load("local r=AO.collectors.bag_capacity();assert(r['0'].equipped_item==nil and r['1'].equipped_item.item_id==8217 and r['1'].equipped_item.inventory_slot==20 and r['2'].equipped_item.item_id==100 and r['3'].equipped_item==nil);assert(itemCalls==4 and actions==0 and accesses==0)").exec()?;
    for setup in [
        "C_Container.ContainerIDToInventoryID=SECRET",
        "C_Container.ContainerIDToInventoryID=nil",
        "GetInventoryItemID=SECRET",
        "GetInventoryItemID=nil",
        "GetInventoryItemID=function() return SECRET end",
        "GetInventoryItemID=function() return nil end",
        "GetInventoryItemID=function() error('PRIVATE') end",
        "C_Container.ContainerIDToInventoryID=function() return SECRET end",
        "C_Container.ContainerIDToInventoryID=function() return 256 end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        lua.load("local r=AO.collectors.bag_capacity();assert(r['1'].total_slots==8 and r['1'].free_slots==2 and r['1'].equipped_item==nil);assert(actions==0 and accesses==0)").exec()?;
    }
    Ok(())
}

#[test]
fn required_guards_and_ambiguous_or_changed_identity() -> mlua::Result<()> {
    let lua = fixture()?;
    for setup in [
        "C_Container=SECRET",
        "C_Container=nil",
        "C_Container=42",
        "C_Container.GetContainerNumSlots=SECRET",
        "C_Container.GetContainerNumFreeSlots=SECRET",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.bag_capacity)")
                .eval::<bool>()?,
            "{setup}"
        );
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
    }
    for setup in [
        "GetInventoryItemID=function() itemCalls=itemCalls+1;return itemCalls%2==1 and 8217 or 100 end",
        "C_Container.ContainerIDToInventoryID=function() return 20 end",
        "local old=GetInventoryItemID;GetInventoryItemID=function(...) local id=old(...);GetInventoryItemID=SECRET;return id end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        lua.load("local r=AO.collectors.bag_capacity();assert(r['1'].equipped_item==nil and r['2'].equipped_item==nil and r['1'].total_slots==8);assert(actions==0 and accesses==0)").exec()?;
    }
    Ok(())
}

#[test]
fn identity_cannot_join_changed_capacity_metadata() -> mlua::Result<()> {
    let lua = fixture()?;
    for field in ["free", "family"] {
        lua.load("reset();changed=false").exec()?;
        lua.load(format!("C_Container.GetContainerNumFreeSlots=function() return {} end;local old=GetInventoryItemID;GetInventoryItemID=function(...) changed=true;return old(...) end",
            if field=="free" {"changed and 3 or 2,0"} else {"2,changed and 32 or 0"})).exec()?;
        lua.load("local r=AO.collectors.bag_capacity();assert(r['1'].equipped_item==nil and r['1'].free_slots==2)").exec()?;
    }
    lua.load("reset();freeReads=0;local old=C_Container.GetContainerNumFreeSlots;C_Container.GetContainerNumFreeSlots=function(bag) if bag==1 then freeReads=freeReads+1;if freeReads==4 then GetInventoryItemID=SECRET end end;return old(bag) end;local r=AO.collectors.bag_capacity();assert(r['1'].equipped_item==nil and accesses==0)").exec()?;
    Ok(())
}

#[test]
fn duplicate_mapping_still_counts_when_item_read_fails() -> mlua::Result<()> {
    let lua = fixture()?;
    for failure in ["return nil", "return SECRET", "error('PRIVATE')"] {
        lua.load("reset();currentBag=0;C_Container.ContainerIDToInventoryID=function(bag) currentBag=bag;return 20 end").exec()?;
        lua.load(format!(
            "GetInventoryItemID=function() if currentBag==2 then {failure} end;return 8217 end"
        ))
        .exec()?;
        lua.load("local r=AO.collectors.bag_capacity();assert(r['1'].equipped_item==nil and r['2'].equipped_item==nil and r['1'].total_slots==8 and actions==0 and accesses==0)").exec()?;
    }
    Ok(())
}
