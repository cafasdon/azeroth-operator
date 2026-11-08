use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn observed_preview_paths_are_optional_stable_and_context_bound()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0;visible=true
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
        FlightMapFrame={IsVisible=function() return visible end,GetMapID=function() return mode=='map_changed' and reads>0 and 2 or 1 end}
        function TakeTaxiNode() error('must not take flight') end
        function TaxiNodeCost(slot) assert(slot==2);return 100 end
        C_TaxiMap={GetAllTaxiNodes=function()
            local middle=mode=='duplicate' and 10 or 30
            return {{nodeID=10,slotIndex=1,name='Origin',state=0,isMapLayerTransition=false},
                {nodeID=20,slotIndex=2,name='Destination',state=1,isMapLayerTransition=false},
                {nodeID=middle,slotIndex=3,name='Transit',state=2,isMapLayerTransition=mode=='transition'}}
        end}
        function GetNumRoutes(slot)
            assert(slot==2);reads=reads+1
            if mode=='zero' then return 0 end
            if mode=='large' then return 65 end
            if mode=='secret_count' then return 'PRIVATE' end
            if mode=='changed_count' and reads>1 then return 1 end
            if mode=='error' then error('PRIVATE') end
            return mode=='direct' and 1 or 2
        end
        function TaxiGetNodeSlot(slot,index,source)
            assert(slot==2)
            if mode=='secret_slot' then return 'PRIVATE' end
            if mode=='unknown_slot' then return 4000 end
            if mode=='closed' then visible=false end
            if mode=='direct' then return source and 1 or 2 end
            if mode=='changed' and reads>1 then return source and 1 or 2 end
            if mode=='disconnected' and index==2 and source then return 1 end
            if mode=='cycle' and index==2 and not source then return 1 end
            if mode=='wrong_end' and index==2 and not source then return 3 end
            if index==1 then return source and 1 or 3 end
            return source and 3 or 2
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads=0;visible=true;AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let value = capture("normal")?;
    assert_eq!(
        value.sections["flight_map"]["entries"]["2"]["observed_route_node_ids"],
        serde_json::json!({"1":10,"2":30,"3":20})
    );
    assert_eq!(
        capture("direct")?.sections["flight_map"]["entries"]["2"]["observed_route_node_ids"],
        serde_json::json!({"1":10,"2":20})
    );
    for mode in [
        "zero",
        "large",
        "secret_count",
        "secret_slot",
        "unknown_slot",
        "changed_count",
        "changed",
        "error",
        "disconnected",
        "cycle",
        "wrong_end",
        "duplicate",
        "transition",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["flight_map"]["entries"]["2"]
                .get("observed_route_node_ids")
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            value.sections["flight_map"]["entries"]["2"]["fare_copper"],
            100
        );
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    for mode in ["closed", "map_changed"] {
        let value = capture(mode)?;
        assert!(!value.sections.contains_key("flight_map"), "{mode}");
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.load("GetNumRoutes=nil").exec()?;
    assert!(
        capture("normal")?.sections["flight_map"]["entries"]["2"]
            .get("observed_route_node_ids")
            .is_none()
    );
    Ok(())
}
