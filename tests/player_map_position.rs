use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;

fn capture(mode: &str, x: f64, y: f64) -> Result<(Snapshot,Lua),Box<dyn std::error::Error>> {
    let lua=Lua::new();lua.globals().set("mode",mode)?;
    lua.globals().set("px",x)?;lua.globals().set("py",y)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};SECRET={};actions=0;reads=0;maps=0
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player');return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function forbidden() actions=actions+1;error('Forbidden action or other unit') end
        C_Map={}
        function C_Map.GetBestMapForUnit(unit)
            assert(unit=='player');maps=maps+1
            if mode=='map_change' and maps>=2 then return 2 end
            if mode=='map_nil' then return nil end
            if mode=='map_secret' then return SECRET end
            if mode=='map_zero' then return 0 end
            if mode=='map_large' then return 4294967296 end
            return 1
        end
        function C_Map.GetPlayerMapPosition(map,unit)
            assert(map==1 and unit=='player');reads=reads+1
            if mode=='nil' then return nil end
            if mode=='secret' then return SECRET end
            if mode=='error' then error('PRIVATE') end
            if mode=='replace_api' then C_Map.GetBestMapForUnit=function() return 1 end end
            if mode=='replace_table' then C_Map={} end
            if mode=='replace_family' then AO.family='retail' end
            if mode=='replace_guard' then issecretvalue=function(v) return v==SECRET end end
            local vector={}
            function vector:GetXY()
                if mode=='coordinate_secret' then return SECRET,py end
                if mode=='coordinate_nil' then return nil,py end
                if mode=='coordinate_string' then return 'PRIVATE',py end
                if mode=='coordinate_nan' then return 0/0,py end
                if mode=='coordinate_inf' then return math.huge,py end
                if mode=='coordinate_negative' then return -0.1,py end
                if mode=='coordinate_large' then return 1.1,py end
                if mode=='moving' and reads==2 then return px+0.1,py end
                if mode=='replace_vector' then self.GetXY=function() return px,py end end
                return px,py,SECRET,'PRIVATE'
            end
            if mode=='missing_xy' then vector.GetXY=nil end
            return vector,SECRET,'PRIVATE'
        end
        if mode=='missing' then C_Map.GetPlayerMapPosition=nil end
        if mode=='guard_mutates_context' then
            function issecretvalue(v)
                if rawequal(v,C_Map.GetPlayerMapPosition) then AO.family='retail' end
                return rawequal(v,SECRET)
            end
        end
        C_Map.OpenWorldMap=forbidden;C_Map.SetUserWaypoint=forbidden;C_Map.RequestPreloadMap=forbidden
        MoveForwardStart=forbidden;UnitPosition=forbidden;UnitGUID=forbidden
    "#).exec()?;
    let ns:mlua::Table=lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua")).call::<()>(("AzerothOperator",ns.clone()))?;
    lua.load(include_str!("../addon/Core.lua")).call::<()>(("AzerothOperator",ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported:String=lua.globals().get("AzerothOperatorExport")?;
    let saved=decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?,0);
    Ok((saved,lua))
}

#[test]
fn player_only_coordinates_survive_full_export() -> Result<(),Box<dyn std::error::Error>> {
    for (x,y) in [(0.0,0.0),(1.0,1.0),(0.25,0.75)] {
        let (saved,lua)=capture("normal",x,y)?;
        let position=&saved.sections["player_map_position"];
        assert_eq!(position["coverage"],"reported_player_map_position");
        assert_eq!(position["map_id"],1);
        assert_eq!(position["x"].as_f64(),Some(x));
        assert_eq!(position["y"].as_f64(),Some(y));
        assert_eq!(lua.globals().get::<u32>("reads")?,2);
        assert_eq!(lua.globals().get::<u32>("maps")?,4);
    }
    Ok(())
}

#[test]
fn unknown_restricted_or_changing_map_data_never_becomes_zero() -> Result<(),Box<dyn std::error::Error>> {
    for mode in ["map_change","map_nil","map_secret","map_zero","map_large","nil","secret","error",
        "replace_api","replace_table","replace_family","replace_guard","coordinate_secret","coordinate_nil",
        "coordinate_string","coordinate_nan","coordinate_inf","coordinate_negative","coordinate_large",
        "moving","replace_vector","missing_xy","missing","guard_mutates_context"] {
        let (saved,_)=capture(mode,0.25,0.75)?;
        assert!(!saved.sections.contains_key("player_map_position"),"{mode}");
        assert_eq!(saved.unavailable["player_map_position"],"api_unavailable_or_failed","{mode}");
    }
    assert!(capture("combat",0.25,0.75).is_err());
    Ok(())
}
