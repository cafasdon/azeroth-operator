use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str, mask: u32) -> Result<(Snapshot, Lua), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.globals().set("mask", mask)?;
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
        function forbidden() actions=actions+1;error('Forbidden PvP action or other-unit query') end
        local names={'UnitIsPVP','UnitIsPVPFreeForAll','UnitIsPVPSanctuary'}
        for index,name in ipairs(names) do
            _G[name]=function(unit)
                assert(unit=='player');reads=reads+1
                local value=math.floor(mask/(2^(index-1)))%2==1
                if mode=='changing' and reads>3 then return not value end
                if mode=='restricted' then return SECRET end
                if mode=='nil' then return nil end
                if mode=='number' then return 0 end
                if mode=='string' then return 'PRIVATE' end
                if mode=='error' then error('PRIVATE') end
                if mode=='replace_api' then _G[name]=function() return value end end
                if mode=='replace_family' then AO.family='retail' end
                if mode=='replace_guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
                return value,SECRET,'PRIVATE'
            end
        end
        if mode=='missing' then UnitIsPVPSanctuary=nil end
        if mode=='bad_guard' then issecretvalue={} end
        if mode=='guard_mutates_context' then
            function issecretvalue(v)
                if rawequal(v,UnitIsPVPSanctuary) then AO.family='retail' end
                return rawequal(v,SECRET)
            end
        end
        TogglePVP=forbidden;SetPVP=forbidden;UnitCanAttack=forbidden;UnitGUID=forbidden
        GetZonePVPInfo=forbidden;AcceptBattlefieldPort=forbidden;LeaveBattlefield=forbidden
    "#).exec()?;
    let ns: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", ns.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let saved = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok((saved, lua))
}

#[test]
fn all_raw_combinations_survive_export_without_normalization()
-> Result<(), Box<dyn std::error::Error>> {
    for mask in 0..8 {
        let (saved, lua) = capture("normal", mask)?;
        assert_eq!(
            saved.sections["player_pvp_flags"],
            json!({"coverage":"reported_player_pvp_flags",
            "is_pvp":mask&1!=0,"is_free_for_all":mask&2!=0,"is_sanctuary":mask&4!=0})
        );
        assert_eq!(lua.globals().get::<u32>("reads")?, 6);
    }
    Ok(())
}

#[test]
fn unknown_restricted_or_changing_flags_never_become_false()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "changing",
        "restricted",
        "nil",
        "number",
        "string",
        "error",
        "replace_api",
        "replace_family",
        "replace_guard",
        "missing",
        "guard_mutates_context",
    ] {
        let (saved, _) = capture(mode, 0)?;
        assert!(!saved.sections.contains_key("player_pvp_flags"), "{mode}");
        assert_eq!(
            saved.unavailable["player_pvp_flags"],
            "api_unavailable_or_failed"
        );
    }
    assert!(capture("combat", 0).is_err());
    assert!(capture("bad_guard", 0).is_err());
    Ok(())
}
