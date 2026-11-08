use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn aura_capture_keeps_only_stable_public_nonprivate_lists() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads={};contexts=0
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function UnitExists(unit)
            assert(unit=='player');contexts=contexts+1
            if mode=='secret_context' then return 'PRIVATE' end
            return mode~='absent' and not(mode=='vanished' and contexts>1)
        end
        function UnitIsVisible(unit) assert(unit=='player');return mode~='invisible' end
        C_UnitAuras={AuraIsPrivate=function(id)
            if id==100 and mode=='private' then return true end
            if id==100 and mode=='unknown_private' then return nil end
            if id==100 and mode=='secret_private' then return 'PRIVATE' end
            return false
        end,
        GetUnitAuras=function(unit,filter,limit)
            assert(unit=='player' and (filter=='HELPFUL' or filter=='HARMFUL') and limit==257)
            reads[filter]=(reads[filter] or 0)+1
            if mode=='both_error' or filter=='HELPFUL' and (mode=='error' or mode=='second_error' and reads[filter]>1) then error('PRIVATE') end
            if filter=='HARMFUL' then return {} end
            if mode=='empty' then return {} end
            if mode=='nil_list' then return nil end
            if mode=='secret_list' then return 'PRIVATE' end
            if mode=='sparse' then return {[1]={spellId=100},[3]={spellId=101}} end
            if mode=='map' then return {x={spellId=100}} end
            if mode=='oversize' then local t={};for i=1,257 do t[i]={spellId=100} end;return t end
            if mode=='changed_count' and reads[filter]>1 then return {} end
            local id=mode=='zero_id' and 0 or mode=='large_id' and 4294967296 or mode=='secret_id' and 'PRIVATE'
                or mode=='changed_id' and reads[filter]>1 and 101 or 100
            local apps=mode=='secret_apps' and 'PRIVATE' or mode=='invalid_apps' and -1
                or mode=='changed_apps' and reads[filter]>1 and 3 or 0
            return {{spellId=id,applications=apps,name='PRIVATE',sourceUnit='PRIVATE',auraInstanceID='PRIVATE'},
                    {spellId=100,applications=2,name='PRIVATE'}}
        end,
        CancelAuraByInstanceID=function() error('no cancellation') end,
        SwitchAuraDataProvider=function() error('no provider changes') end}
        function CastSpellByID() error('no casting') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads={};contexts=0;AO.Capture()").exec()?;
        let value: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{value}\"").as_bytes(),
        )?)
    };
    let normal = capture("normal")?;
    assert_eq!(
        normal.sections["player_auras"]["groups"]["helpful"]["entries"],
        serde_json::json!({
        "1":{"spell_id":100,"applications":0},"2":{"spell_id":100,"applications":2}})
    );
    let summary = azeroth_operator::analysis::summary(&normal);
    assert_eq!(
        summary["player_aura_observations"]["helpful"]["observed_rows"],
        2
    );
    assert_eq!(
        summary["player_aura_observations"]["helpful"]["distinct_spell_ids"],
        1
    );
    assert_eq!(
        summary["player_aura_observations"]["harmful"]["observed_rows"],
        0
    );
    assert!(!serde_json::to_string(&normal)?.contains("PRIVATE"));
    for mode in [
        "private",
        "unknown_private",
        "secret_private",
        "error",
        "second_error",
        "nil_list",
        "secret_list",
        "sparse",
        "map",
        "oversize",
        "changed_count",
        "zero_id",
        "large_id",
        "secret_id",
        "changed_id",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["player_auras"]["groups"]
                .get("helpful")
                .is_none(),
            "{mode}"
        );
        assert!(
            value.sections["player_auras"]["groups"]
                .get("harmful")
                .is_some(),
            "{mode}"
        );
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    for mode in ["secret_apps", "invalid_apps", "changed_apps"] {
        let value = capture(mode)?;
        assert!(
            value.sections["player_auras"]["groups"]["helpful"]["entries"]["1"]
                .get("applications")
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            value.sections["player_auras"]["groups"]["helpful"]["entries"]["2"]["applications"],
            2
        );
    }
    assert_eq!(
        capture("empty")?.sections["player_auras"]["groups"]["helpful"]["entries"],
        serde_json::json!({})
    );
    for mode in [
        "both_error",
        "absent",
        "invisible",
        "vanished",
        "secret_context",
    ] {
        let value = capture(mode)?;
        assert!(!value.sections.contains_key("player_auras"), "{mode}");
        assert!(value.unavailable.contains_key("player_auras"), "{mode}");
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.load("C_UnitAuras.AuraIsPrivate=nil").exec()?;
    assert!(!capture("normal")?.sections.contains_key("player_auras"));
    assert!(capture("combat").is_err());
    Ok(())
}
