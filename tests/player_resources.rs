use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn resources_keep_only_stable_public_player_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal'
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function CastSpellByID() error('no casting') end
        function UseItemByName() error('no items') end
        function TargetUnit() error('no targeting') end
        function UnitHealth(unit,predicted)
            assert(unit=='player' and predicted==false);healthReads=healthReads+1
            if mode=='health_error' or mode=='both_error' or (mode=='health_second_error' and healthReads>1) then error('PRIVATE') end
            if mode=='health_secret' then return 'PRIVATE' end
            if mode=='health_partial' then return nil end
            if mode=='health_float' then return 50.5 end
            if mode=='health_negative' then return -1 end
            if mode=='health_large' then return 4294967296 end
            if mode=='health_nan' then return 0/0 end
            if mode=='health_infinity' then return math.huge end
            if mode=='health_overmax' then return 101 end
            if mode=='health_change' and healthReads>1 then return 49 end
            if mode=='health_zero' then return 0 end
            return 50
        end
        function UnitHealthMax(unit)
            assert(unit=='player')
            if mode=='health_secret_max' then return 'PRIVATE' end
            if mode=='health_maxchange' and healthReads>1 then return 101 end
            if mode=='health_zero' then return 0 end
            return 100
        end
        function UnitPowerType(unit)
            assert(unit=='player');typeReads=typeReads+1
            if mode=='power_secret_type' then return 'PRIVATE' end
            if mode=='power_badtype' then return 64 end
            if mode=='power_typechange' then return typeReads%2 end
            if mode=='power_typechange_between' and typeReads>2 then return 3 end
            if mode=='power_type63' then return 63,'PRIVATE' end
            return 0,'PRIVATE'
        end
        function UnitPower(unit,kind,raw)
            assert(unit=='player' and raw==true and kind>=0 and kind<=63);powerReads=powerReads+1
            if mode=='power_error' or mode=='both_error' or (mode=='power_second_error' and powerReads>1) then error('PRIVATE') end
            if mode=='power_secret' then return 'PRIVATE' end
            if mode=='power_partial' then return nil end
            if mode=='power_float' then return 20.5 end
            if mode=='power_negative' then return -1 end
            if mode=='power_large' then return 4294967296 end
            if mode=='power_nan' then return 0/0 end
            if mode=='power_infinity' then return math.huge end
            if mode=='power_overmax' then return 201 end
            if mode=='power_change' and powerReads>1 then return 21 end
            if mode=='power_zero' then return 0 end
            return 20
        end
        function UnitPowerMax(unit,kind,raw)
            assert(unit=='player' and raw==true and kind>=0 and kind<=63)
            if mode=='power_secret_max' then return 'PRIVATE' end
            if mode=='power_maxchange' and powerReads>1 then return 201 end
            if mode=='power_zero' then return 0 end
            return 200
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("healthReads=0;powerReads=0;typeReads=0;AO.Capture()")
            .exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    let initial = capture("normal")?;
    assert_eq!(
        initial.sections["player_resources"],
        serde_json::json!({
        "coverage":"readable_player_resources","health":{"current":50,"maximum":100},
        "primary_power":{"current":20,"maximum":200,"power_type":0,"units":"unmodified"}})
    );
    assert!(!serde_json::to_string(&initial)?.contains("PRIVATE"));
    for mode in [
        "health_error",
        "health_second_error",
        "health_secret",
        "health_secret_max",
        "health_partial",
        "health_float",
        "health_negative",
        "health_large",
        "health_nan",
        "health_infinity",
        "health_overmax",
        "health_change",
        "health_maxchange",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["player_resources"].get("health").is_none(),
            "{mode}"
        );
        assert!(
            value.sections["player_resources"]
                .get("primary_power")
                .is_some(),
            "{mode}"
        );
    }
    for mode in [
        "power_error",
        "power_second_error",
        "power_secret",
        "power_secret_max",
        "power_partial",
        "power_float",
        "power_negative",
        "power_large",
        "power_nan",
        "power_infinity",
        "power_overmax",
        "power_change",
        "power_maxchange",
        "power_typechange",
        "power_typechange_between",
        "power_badtype",
        "power_secret_type",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["player_resources"]
                .get("primary_power")
                .is_none(),
            "{mode}"
        );
        assert!(
            value.sections["player_resources"].get("health").is_some(),
            "{mode}"
        );
    }
    for (mode, field) in [("health_zero", "health"), ("power_zero", "primary_power")] {
        let value = capture(mode)?;
        let summary = azeroth_operator::analysis::summary(&value);
        assert_eq!(summary["player_resource_observations"][field]["missing"], 0);
        assert!(summary["player_resource_observations"][field]["percent"].is_null());
    }
    assert_eq!(
        capture("power_type63")?.sections["player_resources"]["primary_power"]["power_type"],
        63
    );
    let failed = capture("both_error")?;
    assert!(!failed.sections.contains_key("player_resources"));
    assert!(failed.unavailable.contains_key("player_resources"));
    assert_eq!(failed.sections["equipment"]["16"]["item_id"], 123);
    lua.load("UnitHealth=nil;UnitHealthMax=nil").exec()?;
    assert!(
        capture("normal")?.sections["player_resources"]
            .get("health")
            .is_none()
    );
    lua.load("UnitPower=nil").exec()?;
    assert!(!capture("normal")?.sections.contains_key("player_resources"));
    assert!(capture("combat").is_err());
    Ok(())
}
