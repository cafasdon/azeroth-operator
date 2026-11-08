use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn public_flight_positions_remain_optional_and_guarded() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal'
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={FlightPathState={Current=0,Reachable=1,Unreachable=2}}
        FlightMapFrame={IsVisible=function() return true end,GetMapID=function() return 1 end}
        function TakeTaxiNode() error('must not take flight') end
        function TaxiNodeCost(slot) assert(slot==2);return 100 end
        C_TaxiMap={GetAllTaxiNodes=function(map)
            assert(map==1)
            local calls=0
            local position={GetXY=function()
                calls=calls+1
                if mode=='error' then error('PRIVATE') end
                if mode=='secret' or (mode=='second_secret' and calls>1) then return 'PRIVATE',0.75 end
                if mode=='range' then return -0.1,0.75 end
                if mode=='infinity' then return math.huge,0.75 end
                if mode=='nan' then return 0/0,0.75 end
                if mode=='changed' and calls>1 then return 0.5,0.75 end
                return 0.25,0.75
            end}
            if mode=='missing' then position=nil end
            if mode=='secret_vector' then position='PRIVATE' end
            return {{nodeID=10,slotIndex=1,name='Origin',state=0,isMapLayerTransition=false,position={GetXY=function() return 0,1 end}},
                {nodeID=20,slotIndex=2,name='Destination',state=1,isMapLayerTransition=false,position=position}}
        end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["flight_map"]["entries"]["1"]["map_position"],
        serde_json::json!({"x":0,"y":1})
    );
    assert_eq!(
        saved.sections["flight_map"]["entries"]["2"]["map_position"],
        serde_json::json!({"x":0.25,"y":0.75})
    );
    for mode in [
        "missing",
        "secret_vector",
        "secret",
        "second_secret",
        "range",
        "infinity",
        "nan",
        "changed",
        "error",
    ] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(
            saved.sections["flight_map"]["entries"]["2"]
                .get("map_position")
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            saved.sections["flight_map"]["entries"]["2"]["fare_copper"],
            100
        );
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    Ok(())
}
