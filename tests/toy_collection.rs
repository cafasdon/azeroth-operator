use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn toy_view_capture_preserves_filtered_scope_and_fails_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'; reads=0; counts=0; visible=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return mode=='secret' and value=='Toy 1' end
        ToyBox={IsVisible=function()
            visible=visible+1
            return mode~='hidden' and not (mode=='closed_after' and visible>1)
        end}
        C_ToyBox={
            GetNumFilteredToys=function()
                counts=counts+1
                if mode=='empty' then return 0 end
                if mode=='oversized' then return 4097 end
                if mode=='count_change' and counts>1 then return 1 end
                return 2
            end,
            GetToyFromIndex=function(index)
                reads=reads+1
                if mode=='duplicate' then return 1 end
                if mode=='changing' and reads>2 then return index+2 end
                if mode=='invalid_id' then return -1 end
                return index
            end,
            GetToyInfo=function(id) return mode=='wrong_id' and 99 or id,'Toy '..id,0,false end,
            SetFilterString=function() error('mutation forbidden') end,
            SetIsFavorite=function() error('mutation forbidden') end,
            ForceToyRefilter=function() error('mutation forbidden') end,
        }
        function PlayerHasToy(id) return id==1 end
        function UseToy() error('mutation forbidden') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("reads=0;counts=0;visible=0;AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["toy_collection_view"]["coverage"],
        "filtered_journal"
    );
    assert_eq!(saved.sections["toy_collection_view"]["displayed_count"], 2);
    assert_eq!(
        saved.sections["toy_collection_view"]["entries"]["2"]["collected"],
        false
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["collected_toys_in_view"],
        1
    );
    lua.load("mode='empty'").exec()?;
    let empty = capture()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&empty)["toy_rows_observed"],
        0
    );
    for mode in [
        "hidden",
        "closed_after",
        "duplicate",
        "changing",
        "count_change",
        "invalid_id",
        "wrong_id",
        "oversized",
        "secret",
    ] {
        lua.globals().set("mode", mode)?;
        let failed = capture()?;
        assert!(
            !failed.sections.contains_key("toy_collection_view"),
            "{mode}"
        );
        assert!(
            failed.unavailable.contains_key("toy_collection_view"),
            "{mode}"
        );
        assert!(azeroth_operator::analysis::summary(&failed)["toy_rows_observed"].is_null());
    }
    Ok(())
}
