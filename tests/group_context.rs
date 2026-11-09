use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'
        reads=0;actions=0;finalChecks=0;trace={};counts={};leaders={}
        LE_PARTY_CATEGORY_HOME=17; LE_PARTY_CATEGORY_INSTANCE=29
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
        function forbidden() actions=actions+1; error('forbidden group action or observation') end
        function mutate()
            if mutation=='group' then IsInGroup=function() return false end end
            if mutation=='raid' then IsInRaid=function() return false end end
            if mutation=='count' then GetNumGroupMembers=function() return 0 end end
            if mutation=='leader' then UnitIsGroupLeader=function() return false end end
            if mutation=='family' then AO.family='retail' end
            if mutation=='guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
            if mutation=='home' then LE_PARTY_CATEGORY_HOME=33 end
            if mutation=='instance' then LE_PARTY_CATEGORY_INSTANCE=44 end
            if mutation=='collision' then LE_PARTY_CATEGORY_INSTANCE=LE_PARTY_CATEGORY_HOME end
        end
        function issecretvalue(value)
            if mode=='mutate_final_guard' and reads==4 and rawequal(value,GetNumGroupMembers) then
                finalChecks=finalChecks+1
                if finalChecks==2 then mutate() end
            end
            return rawequal(value,SECRET) or value=='PRIVATE'
        end
        local function record(kind,category)
            assert(category==17 or category==29, 'missing/default/mixed category')
            trace[#trace+1]=kind..':'..category
        end
        function IsInGroup(category)
            record('group',category)
            if category==17 and field=='group' and mode=='bad_field' then return badValue end
            if category==17 and field=='group' and mode=='error' then error('PRIVATE') end
            if category==17 and field=='group' and mode=='unstable' and counts[category]==1 then return false end
            return true
        end
        function IsInRaid(category)
            record('raid',category)
            if category==17 and field=='raid' and mode=='bad_field' then return badValue end
            if category==17 and field=='raid' and mode=='error' then error('PRIVATE') end
            if category==17 and field=='raid' and mode=='unstable' and counts[category]==1 then return true end
            return category==29
        end
        function GetNumGroupMembers(category)
            record('count',category);reads=reads+1;counts[category]=(counts[category] or 0)+1
            if mode=='mutate_count' and reads==4 then mutate() end
            if category==17 and field=='count' and mode=='bad_field' then return badValue end
            if category==17 and field=='count' and mode=='error' then error('PRIVATE') end
            if category==17 and field=='count' and mode=='unstable' and counts[category]==2 then return 5 end
            if mode=='zero' then return 0 end
            if mode=='boundary' then return 4294967295 end
            return category==17 and 3 or 41
        end
        function UnitIsGroupLeader(unit,category)
            assert(unit=='player', 'non-player leader query');record('leader',category)
            leaders[category]=(leaders[category] or 0)+1
            if mode=='mutate_leader' and reads==4 then mutate() end
            if category==17 then
                if mode=='bad_leader' then return badValue end
                if mode=='leader_error' then error('PRIVATE') end
                if mode=='unstable_leader' and leaders[category]==2 then return false end
            end
            return category==17
        end
        GetRaidRosterInfo=forbidden; UnitGUID=forbidden; UnitGroupRolesAssigned=forbidden
        UnitIsGroupAssistant=forbidden; GetNumSubgroupMembers=forbidden
        InviteUnit=forbidden; UninviteUnit=forbidden; ConvertToRaid=forbidden
        ConvertToParty=forbidden; DoReadyCheck=forbidden; InitiateRolePoll=forbidden
        SendChatMessage=forbidden; SendAddonMessage=forbidden; LeaveParty=forbidden
        C_PartyInfo={InviteUnit=forbidden,LeaveParty=forbidden,DoCountdown=forbidden}
        hooksecurefunc=forbidden
        function capture() reads=0;finalChecks=0;trace={};counts={};leaders={};AO.Capture() end
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
fn categories_are_separate_two_pass_observations_without_count_inference()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    assert_eq!(
        capture(&lua, "normal")?.sections["group_context"],
        json!({
        "coverage":"reported_player_group_context", "categories":{
            "home":{"in_group":true,"in_raid":false,"reported_member_count":3,"player_is_leader":true},
            "instance":{"in_group":true,"in_raid":true,"reported_member_count":41,"player_is_leader":false}
        }})
    );
    lua.load("assert(table.concat(trace,',')=='group:17,raid:17,count:17,leader:17,group:29,raid:29,count:29,leader:29,group:17,raid:17,count:17,leader:17,group:29,raid:29,count:29,leader:29')").exec()?;
    for (mode, value) in [("zero", 0_u32), ("boundary", u32::MAX)] {
        let snapshot = capture(&lua, mode)?;
        for category in ["home", "instance"] {
            assert_eq!(
                snapshot.sections["group_context"]["categories"][category]["reported_member_count"],
                value
            );
            assert_eq!(
                snapshot.sections["group_context"]["categories"][category]["in_group"],
                true
            );
        }
    }
    lua.load("SlashCmdList.AZEROTHOPERATOR('')").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(
        decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?
            .sections
            .contains_key("group_context")
    );
    Ok(())
}

#[test]
fn unreadable_or_unstable_category_preserves_the_other_category()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for field in ["group", "raid", "count"] {
        lua.globals().set("field", field)?;
        let invalid = if field == "count" {
            vec![
                "nil",
                "SECRET",
                "'PRIVATE'",
                "'3'",
                "true",
                "{}",
                "-1",
                "1.5",
                "0/0",
                "math.huge",
                "4294967296",
            ]
        } else {
            vec!["nil", "SECRET", "'PRIVATE'", "'true'", "0", "1", "{}"]
        };
        for value in invalid {
            lua.load(format!("badValue={value}")).exec()?;
            let snapshot = capture(&lua, "bad_field")?;
            let categories = &snapshot.sections["group_context"]["categories"];
            assert!(categories.get("home").is_none(), "{field}={value}");
            assert_eq!(categories["instance"]["reported_member_count"], 41);
        }
        for mode in ["error", "unstable"] {
            let snapshot = capture(&lua, mode)?;
            assert!(
                snapshot.sections["group_context"]["categories"]
                    .get("home")
                    .is_none(),
                "{field}:{mode}"
            );
            assert!(
                snapshot.sections["group_context"]["categories"]
                    .get("instance")
                    .is_some()
            );
        }
    }
    Ok(())
}

#[test]
fn optional_leader_failure_does_not_remove_required_group_state()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for value in ["nil", "SECRET", "'PRIVATE'", "'true'", "0", "1", "{}"] {
        lua.load(format!("badValue={value}")).exec()?;
        let snapshot = capture(&lua, "bad_leader")?;
        let categories = &snapshot.sections["group_context"]["categories"];
        assert!(categories["home"].get("player_is_leader").is_none());
        assert_eq!(categories["home"]["reported_member_count"], 3);
        assert_eq!(categories["instance"]["player_is_leader"], false);
    }
    for mode in ["leader_error", "unstable_leader"] {
        let snapshot = capture(&lua, mode)?;
        assert!(
            snapshot.sections["group_context"]["categories"]["home"]
                .get("player_is_leader")
                .is_none()
        );
    }
    for value in ["nil", "SECRET", "true", "123"] {
        let lua = fixture()?;
        lua.load(format!("UnitIsGroupLeader={value}")).exec()?;
        let snapshot = capture(&lua, "normal")?;
        for category in ["home", "instance"] {
            assert!(
                snapshot.sections["group_context"]["categories"][category]
                    .get("player_is_leader")
                    .is_none()
            );
        }
    }
    Ok(())
}

#[test]
fn category_constants_are_required_public_distinct_positive_integers()
-> Result<(), Box<dyn std::error::Error>> {
    for value in [
        "nil",
        "SECRET",
        "'PRIVATE'",
        "'17'",
        "true",
        "{}",
        "0",
        "-1",
        "1.5",
        "0/0",
        "math.huge",
        "4294967296",
    ] {
        let lua = fixture()?;
        lua.load(format!("LE_PARTY_CATEGORY_HOME={value}")).exec()?;
        let snapshot = capture(&lua, "normal")?;
        assert!(
            snapshot.sections["group_context"]["categories"]
                .get("home")
                .is_none(),
            "{value}"
        );
        assert_eq!(
            snapshot.sections["group_context"]["categories"]["instance"]["reported_member_count"],
            41
        );
        lua.load("assert(counts[17]==nil and counts[29]==2)")
            .exec()?;
    }
    for setup in [
        "LE_PARTY_CATEGORY_HOME=nil;LE_PARTY_CATEGORY_INSTANCE=nil",
        "LE_PARTY_CATEGORY_INSTANCE=LE_PARTY_CATEGORY_HOME",
    ] {
        let lua = fixture()?;
        lua.load(setup).exec()?;
        let snapshot = capture(&lua, "normal")?;
        assert!(!snapshot.sections.contains_key("group_context"));
        assert_eq!(
            snapshot.unavailable["group_context"],
            "api_unavailable_or_failed"
        );
        lua.load("assert(reads==0)").exec()?;
    }
    Ok(())
}

#[test]
fn missing_required_apis_and_wrong_family_never_claim_group_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    for api in ["IsInGroup", "IsInRaid", "GetNumGroupMembers"] {
        for value in ["nil", "SECRET", "true", "123"] {
            let lua = fixture()?;
            lua.load(format!("{api}={value}")).exec()?;
            assert!(
                !capture(&lua, "normal")?
                    .sections
                    .contains_key("group_context")
            );
            assert_eq!(lua.globals().get::<u32>("reads")?, 0);
        }
    }
    let lua = fixture()?;
    lua.load("AO.family='retail';assert(not pcall(AO.collectors.group_context))")
        .exec()?;
    let snapshot = capture(&lua, "normal")?;
    assert!(!snapshot.sections.contains_key("group_context"));
    assert!(!snapshot.unavailable.contains_key("group_context"));
    lua.load("AO.family='forever'").exec()?;
    assert!(capture(&lua, "combat").is_err());
    assert_eq!(lua.globals().get::<u32>("reads")?, 0);
    Ok(())
}

#[test]
fn final_callbacks_cannot_publish_stale_context_categories_or_leader_flags()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["mutate_count", "mutate_leader", "mutate_final_guard"] {
        for mutation in [
            "group",
            "raid",
            "count",
            "family",
            "guard",
            "home",
            "instance",
            "collision",
            "leader",
        ] {
            let lua = fixture()?;
            lua.globals().set("mutation", mutation)?;
            let snapshot = capture(&lua, mode)?;
            match mutation {
                "home" | "instance" => {
                    let categories = &snapshot.sections["group_context"]["categories"];
                    assert!(categories.get(mutation).is_none(), "{mode}:{mutation}");
                    assert_eq!(categories.as_object().unwrap().len(), 1);
                }
                "leader" => {
                    for category in ["home", "instance"] {
                        assert!(
                            snapshot.sections["group_context"]["categories"][category]
                                .get("player_is_leader")
                                .is_none(),
                            "{mode}"
                        );
                        assert_eq!(
                            snapshot.sections["group_context"]["categories"][category]["in_group"],
                            true
                        );
                    }
                }
                _ => assert!(!snapshot.sections.contains_key("group_context")),
            }
        }
    }
    Ok(())
}
