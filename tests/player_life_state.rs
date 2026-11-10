use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str, mask: u32) -> Result<(Snapshot,Lua),Box<dyn std::error::Error>> {
    let lua=Lua::new();lua.globals().set("mode",mode)?;lua.globals().set("mask",mask)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};reads=0;actions=0;SECRET={}
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player');return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function forbidden() actions=actions+1;error('forbidden life-state action') end
        local names={'UnitIsDead','UnitIsGhost','UnitIsDeadOrGhost','UnitIsFeignDeath'}
        for index,name in ipairs(names) do
            _G[name]=function(unit)
                assert(unit=='player','non-player life query');reads=reads+1
                local value=math.floor(mask/(2^(index-1)))%2==1
                if mode=='changing' and reads>4 then return not value end
                if mode=='restricted' then return SECRET end
                if mode=='nil' then return nil end
                if mode=='number' then return 1 end
                if mode=='string' then return 'PRIVATE' end
                if mode=='error' then error('PRIVATE') end
                if mode=='replace_api' then _G[name]=function() return value end end
                if mode=='replace_family' then AO.family='retail' end
                if mode=='replace_guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
                return value,SECRET,'PRIVATE'
            end
        end
        if mode=='missing' then UnitIsFeignDeath=nil end
        if mode=='bad_guard' then issecretvalue={} end
        RepopMe=forbidden;RetrieveCorpse=forbidden;AcceptResurrect=forbidden
        UseSoulstone=forbidden;CancelSoulstone=forbidden;ResurrectPlayer=forbidden
        UnitGUID=forbidden;UnitFullName=forbidden;GetCorpseMapPosition=forbidden
        hooksecurefunc=forbidden
    "#).exec()?;
    let ns:mlua::Table=lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua")).call::<()>(("AzerothOperator",ns.clone()))?;
    lua.load(include_str!("../addon/Core.lua")).call::<()>(("AzerothOperator",ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported:String=lua.globals().get("AzerothOperatorExport")?;
    let saved=decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?,0);
    Ok((saved,lua))
}

#[test]
fn raw_boolean_combinations_roundtrip_without_life_or_action_inference() -> Result<(),Box<dyn std::error::Error>> {
    for mask in 0..16 {
        let (saved,lua)=capture("normal",mask)?;
        assert_eq!(saved.sections["player_life_state"],json!({"coverage":"reported_player_life_flags",
            "is_dead":mask&1!=0,"is_ghost":mask&2!=0,"is_dead_or_ghost":mask&4!=0,"is_feigning_death":mask&8!=0}));
        assert_eq!(lua.globals().get::<u32>("reads")?,8);
    }
    Ok(())
}

#[test]
fn unreadable_or_changing_flags_never_become_false() -> Result<(),Box<dyn std::error::Error>> {
    for mode in ["changing","restricted","nil","number","string","error","replace_api","replace_family",
        "replace_guard","missing"] {
        let (saved,_)=capture(mode,0)?;
        assert!(!saved.sections.contains_key("player_life_state"),"{mode}");
        assert_eq!(saved.unavailable["player_life_state"],"api_unavailable_or_failed");
    }
    assert!(capture("combat",0).is_err());
    assert!(capture("bad_guard",0).is_err());
    Ok(())
}
