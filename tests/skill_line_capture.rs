use mlua::Lua;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'}
        accesses=0; calls=0
        SECRET=setmetatable({}, {__index=function() accesses=accesses+1;error('PRIVATE') end,
            __call=function() accesses=accesses+1;error('PRIVATE') end})
        function issecretvalue(v) return rawequal(v,SECRET) end
        function reset()
            accesses=0;calls=0
            C_SkillInfo={GetNumSkillLines=function() return 1 end,
                GetSkillLineInfo=function()
                    calls=calls+1
                    return {isHeader=false,skillID=43,name='Fixture',rank=100,maxRank=300,
                        skillLineCategoryID=9,modifier=5,tempPoints=2}
                end}
        end
        reset()
    "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

#[test]
fn restricted_api_is_not_indexed_or_called() -> mlua::Result<()> {
    let lua = fixture()?;
    for setup in [
        "C_SkillInfo=SECRET",
        "C_SkillInfo=nil",
        "C_SkillInfo=42",
        "C_SkillInfo.GetNumSkillLines=SECRET",
        "C_SkillInfo.GetSkillLineInfo=SECRET",
        "C_SkillInfo.GetSkillLineInfo=nil",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.skill_lines)")
                .eval::<bool>()?,
            "{setup}"
        );
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
    }
    Ok(())
}

#[test]
fn stable_projection_required_even_when_count_is_unchanged() -> mlua::Result<()> {
    let lua = fixture()?;
    lua.load("local r=AO.collectors.skill_lines()['43'];assert(r.rank==100 and r.modifier==5 and r.temporary_points==2)").exec()?;
    for field in [
        "rank",
        "maxRank",
        "skillID",
        "skillLineCategoryID",
        "modifier",
        "tempPoints",
    ] {
        lua.load("reset()").exec()?;
        lua.load(format!("local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();if calls>1 then r.{field}=r.{field}+1 end;return r end")).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.skill_lines)")
                .eval::<bool>()?,
            "{field}"
        );
    }
    Ok(())
}

#[test]
fn boundaries_and_pointer_changes() -> mlua::Result<()> {
    let lua = fixture()?;
    lua.load("C_SkillInfo.GetNumSkillLines=function() return 512 end;local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function(i) local r=old();r.skillID=i;r.rank=0;r.maxRank=0;return r end;local r=AO.collectors.skill_lines();assert(r['1'].rank==0 and r['512'].max_rank==0)").exec()?;
    for setup in [
        "C_SkillInfo.GetNumSkillLines=function() return 513 end",
        "C_SkillInfo.GetNumSkillLines=function() return 0 end",
        "local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();C_SkillInfo=SECRET;return r end",
        "local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();C_SkillInfo.GetNumSkillLines=SECRET;return r end",
        "local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();if calls>1 then r.modifier=nil end;return r end",
        "local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();if calls>1 then r.name='Changed' end;return r end",
        "local old=C_SkillInfo.GetSkillLineInfo;C_SkillInfo.GetSkillLineInfo=function() local r=old();if calls>1 then r.isHeader=true end;return r end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        assert!(
            !lua.load("return pcall(AO.collectors.skill_lines)")
                .eval::<bool>()?,
            "{setup}"
        );
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
    }
    lua.load("reset();C_SkillInfo.GetSkillLineInfo=function() return {isHeader=true} end;assert(next(AO.collectors.skill_lines())==nil)").exec()?;
    Ok(())
}
