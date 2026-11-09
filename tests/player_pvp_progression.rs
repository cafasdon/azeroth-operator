use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'
        calls=0; actions=0; finalChecks=0; trace={}
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player'); return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() end
        SECRET=setmetatable({}, {__tostring=function() error('secret inspected') end})
        function forbidden() actions=actions+1; error('unexpected PvP action or getter') end
        function mutate()
            if mutation=='namespace' then C_MajorFactions={} end
            if mutation=='getter' then C_MajorFactions.GetMajorFactionProgressionInfo=function() return nil end end
            if mutation=='family' then AO.family='retail' end
            if mutation=='guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
        end
        function issecretvalue(value)
            if mode=='mutate_final_guard' and calls==2 and rawequal(value,issecretvalue) then
                finalChecks=finalChecks+1
                if finalChecks==2 then mutate() end
            end
            if mode=='mutate_optional_guard' and calls==2 and value==777 then mutate() end
            return rawequal(value,SECRET) or value=='PRIVATE'
        end
        C_MajorFactions={GetMajorFactionProgressionInfo=function(id)
            assert(id==2800, 'wrong faction'); calls=calls+1; trace[#trace+1]=id
            if mode=='mutate_getter' and calls==2 then mutate() end
            if mode=='error' or mode=='second_error' and calls==2 then error('PRIVATE') end
            if mode=='bad_result' then return badValue end
            local data={renownLevel=4,renownReputationEarned=25,renownLevelThreshold=100,
                maxLevel=14,currentWeekProgressiveMaxLevel=8,previousWeekProgressiveMaxLevel=7}
            if mode=='zero' then for key in pairs(data) do data[key]=0 end end
            if mode=='boundary' then for key in pairs(data) do data[key]=4294967295 end end
            if mode=='independent_caps' then
                data.renownReputationEarned=500; data.maxLevel=1
                data.currentWeekProgressiveMaxLevel=0; data.previousWeekProgressiveMaxLevel=1000
            end
            if mode=='bad_field' then data[field]=badValue end
            if mode=='field_error' then data[field]=nil end
            if mode=='changed_field' and calls==2 then data[field]=999 end
            if mode=='missing_second' and calls==2 then data[field]=nil end
            if mode=='mutate_optional_guard' then data.maxLevel=777 end
            return setmetatable(data,{__index=function(_,key)
                if mode=='field_error' and key==field then error('PRIVATE') end
                if key=='renownLevel' or key=='renownReputationEarned' or key=='renownLevelThreshold'
                    or key=='maxLevel' or key=='currentWeekProgressiveMaxLevel'
                    or key=='previousWeekProgressiveMaxLevel' then return nil end
                forbidden()
            end})
        end}
        C_PvP={GetPVPSeasonInfo=forbidden,GetScoreInfo=forbidden,RequestCrowdControlSpell=forbidden}
        C_MajorFactions.GetMajorFactionData=forbidden
        C_MajorFactions.RequestCatchUpState=forbidden
        C_Reputation={GetFactionDataByID=forbidden,RequestFactionParagonPreloadRewardData=forbidden}
        RequestBattlefieldScoreData=forbidden; RequestRatedInfo=forbidden
        JoinBattlefield=forbidden; JoinRatedBattlefield=forbidden; AcceptBattlefieldPort=forbidden
        GetBattlefieldScore=forbidden; GetPVPLifetimeStats=forbidden; hooksecurefunc=forbidden
        function capture() calls=0;trace={};finalChecks=0;AO.Capture() end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua, mode: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
    lua.globals().set("mode", mode)?;
    lua.load("capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok(snapshot)
}

#[test]
fn pvp_projection_reads_only_fixed_faction_twice() -> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let snapshot = capture(&lua, "normal")?;
    assert_eq!(
        snapshot.sections["player_pvp_progression"],
        json!({
            "coverage":"reported_pvp_rank_faction", "faction_id":2800,
            "rank":4, "rank_points":25, "next_rank_threshold":100,
            "max_rank":14, "current_week_max_rank":8, "previous_week_max_rank":7
        })
    );
    lua.load("assert(calls==2 and table.concat(trace,',')=='2800,2800')")
        .exec()?;
    for mode in ["zero", "boundary"] {
        let snapshot = capture(&lua, mode)?;
        let expected = if mode == "zero" { 0 } else { u32::MAX };
        for field in [
            "rank",
            "rank_points",
            "next_rank_threshold",
            "max_rank",
            "current_week_max_rank",
            "previous_week_max_rank",
        ] {
            assert_eq!(snapshot.sections["player_pvp_progression"][field], expected);
        }
    }
    let independent = capture(&lua, "independent_caps")?;
    assert_eq!(
        independent.sections["player_pvp_progression"]["max_rank"],
        1
    );
    assert_eq!(
        independent.sections["player_pvp_progression"]["rank_points"],
        500
    );
    lua.load("mode='normal';SlashCmdList.AZEROTHOPERATOR('')")
        .exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(
        decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?
            .sections
            .contains_key("player_pvp_progression")
    );
    Ok(())
}

#[test]
fn required_values_must_be_public_uints_and_stable() -> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for field in [
        "renownLevel",
        "renownReputationEarned",
        "renownLevelThreshold",
    ] {
        lua.globals().set("field", field)?;
        for value in [
            "nil",
            "SECRET",
            "'PRIVATE'",
            "'1'",
            "false",
            "{}",
            "-1",
            "1.5",
            "0/0",
            "math.huge",
            "-math.huge",
            "4294967296",
        ] {
            lua.load(format!("badValue={value}")).exec()?;
            let snapshot = capture(&lua, "bad_field")?;
            assert!(
                !snapshot.sections.contains_key("player_pvp_progression"),
                "{field}={value}"
            );
            assert_eq!(
                snapshot.unavailable["player_pvp_progression"],
                "api_unavailable_or_failed"
            );
            assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
        }
        for mode in ["changed_field", "missing_second", "field_error"] {
            assert!(
                !capture(&lua, mode)?
                    .sections
                    .contains_key("player_pvp_progression"),
                "{field}: {mode}"
            );
        }
    }
    for mode in ["error", "second_error"] {
        assert!(
            !capture(&lua, mode)?
                .sections
                .contains_key("player_pvp_progression")
        );
    }
    for value in ["nil", "SECRET", "'PRIVATE'", "false", "123"] {
        lua.load(format!("badValue={value}")).exec()?;
        assert!(
            !capture(&lua, "bad_result")?
                .sections
                .contains_key("player_pvp_progression")
        );
    }
    Ok(())
}

#[test]
fn optional_fields_fail_independently() -> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let fields = [
        ("maxLevel", "max_rank"),
        ("currentWeekProgressiveMaxLevel", "current_week_max_rank"),
        ("previousWeekProgressiveMaxLevel", "previous_week_max_rank"),
    ];
    for (input, output) in fields {
        lua.globals().set("field", input)?;
        for value in [
            "nil",
            "SECRET",
            "'PRIVATE'",
            "'1'",
            "false",
            "{}",
            "-1",
            "1.5",
            "0/0",
            "math.huge",
            "-math.huge",
            "4294967296",
        ] {
            lua.load(format!("badValue={value}")).exec()?;
            let snapshot = capture(&lua, "bad_field")?;
            let section = &snapshot.sections["player_pvp_progression"];
            assert!(section.get(output).is_none(), "{input}={value}");
            assert_eq!(section["rank"], 4);
            assert_eq!(section.as_object().unwrap().len(), 7);
        }
        for mode in ["changed_field", "missing_second", "field_error"] {
            let snapshot = capture(&lua, mode)?;
            assert!(
                snapshot.sections["player_pvp_progression"]
                    .get(output)
                    .is_none(),
                "{input}: {mode}"
            );
            assert_eq!(
                snapshot.sections["player_pvp_progression"]["rank_points"],
                25
            );
        }
    }
    Ok(())
}

#[test]
fn unavailable_api_family_and_combat_never_claim_coverage() -> Result<(), Box<dyn std::error::Error>>
{
    for api in [
        "C_MajorFactions",
        "C_MajorFactions.GetMajorFactionProgressionInfo",
    ] {
        for value in ["nil", "SECRET", "true", "123"] {
            let lua = fixture()?;
            lua.load(format!("{api}={value}")).exec()?;
            assert!(
                !capture(&lua, "normal")?
                    .sections
                    .contains_key("player_pvp_progression")
            );
            assert_eq!(lua.globals().get::<u32>("calls")?, 0);
        }
    }
    let lua = fixture()?;
    lua.load("issecretvalue=nil").exec()?;
    assert!(
        capture(&lua, "normal")?
            .sections
            .contains_key("player_pvp_progression")
    );
    lua.load("issecretvalue=true; assert(not pcall(AO.collectors.player_pvp_progression))")
        .exec()?;
    let lua = fixture()?;
    lua.load("AO.family='retail'; assert(not pcall(AO.collectors.player_pvp_progression))")
        .exec()?;
    let retail = capture(&lua, "normal")?;
    assert!(!retail.sections.contains_key("player_pvp_progression"));
    assert!(!retail.unavailable.contains_key("player_pvp_progression"));
    lua.load("assert(calls==0); AO.family='forever'").exec()?;
    assert!(capture(&lua, "combat").is_err());
    assert_eq!(lua.globals().get::<u32>("calls")?, 0);
    Ok(())
}

#[test]
fn getter_and_final_restriction_callbacks_cannot_change_context()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "mutate_getter",
        "mutate_optional_guard",
        "mutate_final_guard",
    ] {
        for mutation in ["namespace", "getter", "family", "guard"] {
            let lua = fixture()?;
            lua.globals().set("mutation", mutation)?;
            let snapshot = capture(&lua, mode)?;
            assert!(
                !snapshot.sections.contains_key("player_pvp_progression"),
                "{mode}: {mutation}"
            );
            assert_eq!(
                snapshot.unavailable["player_pvp_progression"],
                "api_unavailable_or_failed"
            );
        }
    }
    Ok(())
}
