use mlua::Lua;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; accesses=0;actions=0;calls=0
        SECRET=setmetatable({}, {__index=function() accesses=accesses+1;error('PRIVATE') end,
            __call=function() accesses=accesses+1;error('PRIVATE') end})
        function issecretvalue(v) return rawequal(v,SECRET) end
        function forbidden() actions=actions+1;error('PRIVATE action') end
        function reset()
            accesses=0;actions=0;calls=0
            GossipFrame={IsVisible=function() return true end}
            C_GossipInfo={SelectOption=forbidden,SelectOptionByIndex=forbidden,
                RefreshOptions=forbidden,CloseGossip=forbidden,
                GetOptions=function()
                    calls=calls+1
                    return {{name='Show your wares',gossipOptionID=42,status=0,flags=1,orderIndex=-1,spellID=123},
                        {name='Tell me more'}}
                end}
        end
        reset()
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

#[test]
fn visible_options_keep_nullable_ids_and_never_act() -> mlua::Result<()> {
    let lua = fixture()?;
    lua.load("local s=AO.collectors.gossip_options();assert(s.coverage=='visible_gossip_options');local r=s.entries['1'];assert(r.name=='Show your wares' and r.option_id==42 and r.reported_status==0 and r.reported_flags==1 and r.reported_order_index==-1 and r.spell_id==123);assert(s.entries['2'].option_id==nil);assert(calls==2 and actions==0 and accesses==0)").exec()?;
    lua.load("reset();C_GossipInfo.GetOptions=function() return {} end;assert(next(AO.collectors.gossip_options().entries)==nil)").exec()?;
    Ok(())
}

#[test]
fn required_restricted_values_fail_without_touching_or_selecting() -> mlua::Result<()> {
    let lua = fixture()?;
    for setup in [
        "C_GossipInfo=SECRET",
        "C_GossipInfo=nil",
        "C_GossipInfo=42",
        "GossipFrame=SECRET",
        "GossipFrame=nil",
        "GossipFrame=42",
        "C_GossipInfo.GetOptions=SECRET",
        "C_GossipInfo.GetOptions=nil",
        "GossipFrame.IsVisible=SECRET",
        "GossipFrame.IsVisible=nil",
        "GossipFrame.IsVisible=function() return false end",
        "GossipFrame.IsVisible=function() return SECRET end",
        "C_GossipInfo.GetOptions=function() return SECRET end",
        "C_GossipInfo.GetOptions=function() return {SECRET} end",
        "C_GossipInfo.GetOptions=function() return {{name=SECRET}} end",
        "C_GossipInfo.GetOptions=function() C_GossipInfo=SECRET;return {} end",
        "C_GossipInfo.GetOptions=function() GossipFrame=SECRET;return {} end",
        "C_GossipInfo.GetOptions=function() return {[2]={name='Sparse'}} end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.gossip_options)")
                .eval::<bool>()?,
            "{setup}"
        );
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
        assert_eq!(lua.globals().get::<u32>("actions")?, 0, "{setup}");
    }
    Ok(())
}

#[test]
fn optional_metadata_and_stability_and_boundaries() -> mlua::Result<()> {
    let lua = fixture()?;
    lua.load("C_GossipInfo.GetOptions=function() return {{name='Readable',gossipOptionID=SECRET,status=SECRET,flags=SECRET,orderIndex=SECRET,spellID=SECRET,rewards=SECRET,failureDescription=SECRET}} end;local r=AO.collectors.gossip_options().entries['1'];assert(r.name=='Readable' and r.option_id==nil and r.reported_status==nil and accesses==0 and actions==0)").exec()?;
    for field in [
        "gossipOptionID",
        "status",
        "flags",
        "orderIndex",
        "spellID",
        "name",
    ] {
        lua.load("reset()").exec()?;
        lua.load(format!("local old=C_GossipInfo.GetOptions;C_GossipInfo.GetOptions=function() local rows=old();if calls>1 then rows[1].{field}=nil end;return rows end")).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.gossip_options)")
                .eval::<bool>()?,
            "{field}"
        );
    }
    lua.load("reset();C_GossipInfo.GetOptions=function() local rows={};for i=1,128 do rows[i]={name=string.rep('x',512)} end;return rows end;assert(AO.collectors.gossip_options().entries['128'].name)").exec()?;
    for setup in [
        "C_GossipInfo.GetOptions=function() local r={};for i=1,129 do r[i]={name='A'} end;return r end",
        "C_GossipInfo.GetOptions=function() return {{name=string.rep('x',513)}} end",
        "local old=C_GossipInfo.GetOptions;C_GossipInfo.GetOptions=function() local r=old();if calls>1 then r[1],r[2]=r[2],r[1] end;return r end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.gossip_options)")
                .eval::<bool>()?,
            "{setup}"
        );
    }
    Ok(())
}
