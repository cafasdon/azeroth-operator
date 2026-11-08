use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn active_pet_public_details_preserve_unknowns_and_reject_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player');return 'Fixture' end
        function UnitGUID() error('must not read GUID') end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function UnitExists(unit) assert(unit=='pet');return mode~='absent' end
        function UnitCreatureFamily(unit)
            assert(unit=='pet');reads=reads+1
            if mode=='error' then error('PRIVATE') end
            if mode=='unknown' then return 'PRIVATE',nil end
            if mode=='secret' then return 'PRIVATE','PRIVATE' end
            if mode=='invalid' then return 'PRIVATE',0 end
            return 'PRIVATE', mode=='changed_family' and reads>1 and 2 or 1
        end
        function UnitLevel(unit)
            assert(unit=='pet')
            if mode=='error' then error('PRIVATE') end
            if mode=='unknown' then return nil end
            if mode=='secret' then return 'PRIVATE' end
            if mode=='invalid' then return -1 end
            return mode=='changed_level' and reads>1 and 18 or 17
        end
        C_PetInfo={GetPetTrainingPoints=function()
            if mode=='training_known' then return 25,10 end
            if mode=='training_zero' then return 0,0 end
            if mode=='training_deficit' then return 10,20 end
            if mode=='training_negative' then return -10,20 end
            if mode=='training_secret' then return 'PRIVATE',10 end
            if mode=='training_partial' then return 25,nil end
            if mode=='training_invalid' then return 2147483648,0 end
            if mode=='training_error' then error('PRIVATE') end
            if mode=='training_changed' then return 25,reads>1 and 11 or 10 end
            return nil,nil
        end}
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
    let saved = capture()?;
    assert_eq!(
        saved.sections["pet_care"],
        serde_json::json!({"present":true,"family_id":1,"level":17})
    );
    for mode in ["unknown", "secret", "invalid", "error"] {
        lua.globals().set("mode", mode)?;
        assert_eq!(
            capture()?.sections["pet_care"],
            serde_json::json!({"present":true})
        );
    }
    for (mode, total, used) in [
        ("training_known", 25, 10),
        ("training_zero", 0, 0),
        ("training_deficit", 10, 20),
        ("training_negative", -10, 20),
    ] {
        lua.globals().set("mode", mode)?;
        assert_eq!(
            capture()?.sections["pet_care"]["training_points"],
            serde_json::json!({"total":total,"used":used})
        );
    }
    for mode in [
        "training_secret",
        "training_partial",
        "training_invalid",
        "training_error",
    ] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(saved.sections["pet_care"].get("training_points").is_none());
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in ["changed_family", "changed_level", "training_changed"] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(saved.unavailable.contains_key("pet_care"));
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("mode='absent'").exec()?;
    assert_eq!(
        capture()?.sections["pet_care"],
        serde_json::json!({"present":false})
    );
    assert_eq!(lua.globals().get::<u32>("reads")?, 0);
    lua.load("mode='normal';UnitCreatureFamily=nil;UnitLevel=nil")
        .exec()?;
    assert_eq!(
        capture()?.sections["pet_care"],
        serde_json::json!({"present":true})
    );
    Ok(())
}
