use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn mount_collection_capture_is_bounded_and_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'; calls=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return mode=='secret' and value==40002 end
        C_MountJournal={
            GetMountIDs=function()
                calls=calls+1
                if mode=='duplicate' then return {2,2} end
                if mode=='sparse' then return {[2]=2} end
                if mode=='empty_catalogue' then return {} end
                if mode=='changing' and calls%2==0 then return {2,7,10} end
                return {2,7,9}
            end,
            GetMountInfoByID=function(id)
                local collected=mode~='unowned' and id~=7
                local returned=mode=='wrong_id' and 99 or id
                return 'Mount '..id,40000+id,0,false,true,0,false,false,nil,false,collected,returned,false
            end,
            SummonByID=function() error('mutation forbidden') end,
            SetIsFavorite=function() error('mutation forbidden') end,
            SetCollectedFilterSetting=function() error('mutation forbidden') end,
        }
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("calls=0; AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(saved.sections["mount_collection"]["catalogue_count"], 3);
    assert_eq!(
        saved.sections["mount_collection"]["entries"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        saved.sections["mount_collection"]["entries"]["2"]["spell_id"],
        40002
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["collected_mounts_observed"],
        2
    );
    lua.load("mode='unowned'").exec()?;
    let empty = capture()?;
    assert!(
        empty.sections["mount_collection"]["entries"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&empty)["collected_mounts_observed"],
        0
    );
    for mode in [
        "duplicate",
        "sparse",
        "empty_catalogue",
        "changing",
        "wrong_id",
        "secret",
    ] {
        lua.globals().set("mode", mode)?;
        let failed = capture()?;
        assert!(!failed.sections.contains_key("mount_collection"), "{mode}");
        assert!(
            failed.unavailable.contains_key("mount_collection"),
            "{mode}"
        );
        assert!(
            azeroth_operator::analysis::summary(&failed)["collected_mounts_observed"].is_null()
        );
    }
    lua.load("C_MountJournal=nil").exec()?;
    assert!(capture()?.unavailable.contains_key("mount_collection"));
    Ok(())
}
