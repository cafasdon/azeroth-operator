use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn durability_capture_retains_only_readable_stable_pairs() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function RepairAllItems() error('no repair action') end
        function GetInventoryItemDurability(slot)
            assert(slot>=1 and slot<=19)
            reads[slot]=(reads[slot] or 0)+1
            if slot==1 then return 0,100 end
            if slot==5 then
                if mode=='error' then error('PRIVATE') end
                if mode=='secret' then return 'PRIVATE',100 end
                if mode=='secret_max' then return 50,'PRIVATE' end
                if mode=='partial' then return 50 end
                if mode=='zero_max' then return 0,0 end
                if mode=='over_max' then return 101,100 end
                if mode=='negative' then return -1,100 end
                if mode=='float' then return 50.5,100 end
                if mode=='overflow' then return 4294967296,4294967296 end
                if mode=='nan' then return 0/0,100 end
                if mode=='infinity' then return 50,math.huge end
                if mode=='changed' and reads[slot]>1 then return 49,100 end
                if mode=='changed_max' and reads[slot]>1 then return 50,101 end
                if mode=='second_error' and reads[slot]>1 then error('PRIVATE') end
                if mode=='second_secret' and reads[slot]>1 then return 50,'PRIVATE' end
                return 50,100
            end
            if slot==16 then return 75,75 end
        end
    "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads={};AO.Capture()").exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    let value = capture("normal")?;
    assert_eq!(
        value.sections["equipment_durability"]["entries"],
        serde_json::json!({"1":{"current":0,"maximum":100},"5":{"current":50,"maximum":100},"16":{"current":75,"maximum":75}})
    );
    let condition = azeroth_operator::analysis::summary(&value)["equipment_condition"].clone();
    assert_eq!(condition["observed_slots"], 3);
    assert_eq!(condition["unobserved_slots"], 16);
    assert_eq!(condition["damaged_slots"], serde_json::json!([1, 5]));
    assert_eq!(condition["broken_slots"], serde_json::json!([1]));
    for mode in [
        "error",
        "secret",
        "secret_max",
        "partial",
        "zero_max",
        "over_max",
        "negative",
        "float",
        "overflow",
        "nan",
        "infinity",
        "changed",
        "changed_max",
        "second_error",
        "second_secret",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["equipment_durability"]["entries"]
                .get("5")
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            value.sections["equipment_durability"]["entries"]["1"]["current"],
            0
        );
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    lua.load("GetInventoryItemDurability=function() end")
        .exec()?;
    let empty = capture("normal")?;
    assert_eq!(
        empty.sections["equipment_durability"]["entries"],
        serde_json::json!({})
    );
    assert!(azeroth_operator::analysis::summary(&empty)["equipment_condition"]["minimum_observed_percent"].is_null());
    lua.load("GetInventoryItemDurability=nil").exec()?;
    let absent = capture("normal")?;
    assert!(absent.unavailable.contains_key("equipment_durability"));
    assert_eq!(absent.sections["equipment"]["16"]["item_id"], 123);
    Ok(())
}
