use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn incomplete_profession_data_does_not_invalidate_other_sections()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function GetProfessions()
            reads=reads+1
            if mode=='empty' then return nil,nil,nil,nil,nil,nil end
            if mode=='bad_index' then return nil,'bad',nil,4,5,nil end
            if mode=='secret_index' then return nil,'PRIVATE',nil,4,5,nil end
            if mode=='changed_ids' and reads>1 then return nil,3,nil,4,5,nil end
            if mode=='removed' and reads>1 then return nil,nil,nil,4,5,nil end
            if mode=='added' and reads>1 then return 1,2,nil,4,5,nil end
            return nil,2,nil,4,5,nil
        end
        function GetProfessionInfo(index)
            assert(type(index)=='number','invalid index reached API')
            if mode=='unloaded' and index==2 then return nil end
            if mode=='error' then error('PRIVATE') end
            local name='Profession '..index
            if nameBytes then name=nameBytes end
            local rank=3
            local cap=75
            local skill=index==2 and 164 or (index==4 and 356 or (index==5 and 185 or 333))
            if mode=='bad_rank' then rank=true end
            if mode=='negative_rank' then rank=-1 end
            if mode=='secret_rank' then rank='PRIVATE' end
            if mode=='bad_name' then name=string.rep('x',16385) end
            if mode=='invalid_utf8' then name=string.char(255) end
            if mode=='bad_cap' then cap=4294967296 end
            if mode=='bad_skill' then skill=0 end
            if mode=='secret_name' then name='PRIVATE' end
            if mode=='changed_rank' and reads>1 then rank=4 end
            return name,'PRIVATE',rank,cap,nil,nil,skill
        end
    "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("reads=0;AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let good = capture()?;
    assert_eq!(good.sections["professions"].as_object().unwrap().len(), 3);
    assert_eq!(good.sections["professions"]["2"]["skill_line"], 164);
    assert!(good.sections["professions"].get("1").is_none());
    lua.load("AO.family='retail'").exec()?;
    assert_eq!(
        capture()?.sections["professions"],
        good.sections["professions"]
    );
    lua.load("AO.family='forever'").exec()?;
    for bytes in [
        vec![0],
        vec![0xc3, 0xa9],
        vec![0xe0, 0xa0, 0x80],
        vec![0xed, 0x9f, 0xbf],
        vec![0xf4, 0x8f, 0xbf, 0xbf],
    ] {
        lua.globals().set("nameBytes", lua.create_string(bytes)?)?;
        assert!(capture()?.sections.contains_key("professions"));
    }
    for bytes in [
        vec![0x80],
        vec![0xc3],
        vec![0xc0, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
    ] {
        lua.globals().set("nameBytes", lua.create_string(bytes)?)?;
        let saved = capture()?;
        assert!(saved.unavailable.contains_key("professions"));
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.globals().set("nameBytes", mlua::Value::Nil)?;
    lua.load("mode='empty'").exec()?;
    assert!(
        capture()?.sections["professions"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    for mode in [
        "unloaded",
        "bad_index",
        "secret_index",
        "error",
        "bad_rank",
        "negative_rank",
        "secret_rank",
        "bad_name",
        "invalid_utf8",
        "bad_cap",
        "bad_skill",
        "secret_name",
        "changed_ids",
        "changed_rank",
        "added",
        "removed",
    ] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(saved.unavailable.contains_key("professions"), "{mode}");
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123, "{mode}");
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    Ok(())
}
