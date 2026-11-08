use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn pet_view_aggregates_species_without_individual_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE-GUID' or v=='PRIVATE-CUSTOM' or v=='SECRET-FLAG'
            or (mode=='secret_species' and v==7) end
        PetJournal={IsVisible=function() return mode~='hidden' and not(mode=='hidden_end' and reads>1) end}
        C_PetJournal={GetNumPets=function()
                reads=reads+1
                if mode=='empty' then return 0,999 end
                if mode=='oversize' then return 10001,999 end
                if mode=='changed_count' and reads>1 then return 2,999 end
                return 3,999
            end,
            GetPetInfoByIndex=function(index)
                local owned=index<3
                if mode=='bad_flag' then owned=nil end
                if mode=='secret_flag' then owned='SECRET-FLAG' end
                if mode=='changed_row' and reads>1 and index==1 then owned=false end
                return 'PRIVATE-GUID',index<3 and 7 or 8,owned,'PRIVATE-CUSTOM'
            end,
            GetPetInfoBySpeciesID=function(id)
                if mode=='bad_name' then return nil end
                return 'Species '..id
            end,
            SetSearchFilter=function() error('filter forbidden') end,
            SummonPetByGUID=function() error('summon forbidden') end,
            SetCustomName=function() error('rename forbidden') end,
            ReleasePetByID=function() error('release forbidden') end,
            CagePetByID=function() error('cage forbidden') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("reads=0;AO.Capture()").exec()?;
        let encoded: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{encoded}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["pet_collection_view"]["entries"]["7"],
        serde_json::json!({"name":"Species 7","owned_rows":2,"unowned_rows":0})
    );
    assert_eq!(
        saved.sections["pet_collection_view"]["entries"]["8"]["unowned_rows"],
        1
    );
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE-"));
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["pet_collection_view_counts"],
        serde_json::json!({
        "displayed_rows":3,"species_observed":2,"owned_rows_observed":2,"unowned_rows_observed":1})
    );
    lua.load("mode='empty'").exec()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&capture()?)["pet_collection_view_counts"]["species_observed"],
        0
    );
    for mode in [
        "hidden",
        "hidden_end",
        "oversize",
        "changed_count",
        "changed_row",
        "secret_species",
        "bad_flag",
        "secret_flag",
        "bad_name",
    ] {
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?.unavailable.contains_key("pet_collection_view"),
            "{mode}"
        );
    }
    Ok(())
}
