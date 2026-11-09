use azeroth_operator::import::decode;
use mlua::Lua;
use serde_json::{Value, json};

fn fixture() -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        secret={}
        function issecretvalue(v) return v==secret end
        function forbidden() forbiddenCalls=forbiddenCalls+1;error('Unexpected action or private lookup') end
        CastSpellByName=forbidden; MoveForwardStart=forbidden; UnitGUID=forbidden
        UnitHealth=forbidden; C_Map=setmetatable({},{__index=forbidden})
        function setup()
            calls={Npc={},Object={}}; spawnCalls=0; itemCalls=0;forbiddenCalls=0
            input={[7]={{50,60,0},{-1,-1},{50,60,2},{50,60},{0,100}},[2]={{1,2}}}
            LibQuestieDB={
                RequireContract=function(v) assert(v==1);return true end,
                ModeIndicator={GetStatus=function() return {
                    expansion='Forever',mode='baked',
                    contractVersion=mode=='context_change' and spawnCalls>0 and 5 or 4} end},
                Item={Exists=function(id) return mode~='item_unknown' end,Get=function(id,key)
                    itemCalls=itemCalls+1
                    if mode=='item_change' and key=='npcDrops' then return {itemCalls} end
                    if mode=='budget' then
                        local result={}
                        local offset=key=='vendors' and 10000 or 0
                        if key=='npcDrops' or key=='objectDrops' or key=='vendors' then
                            for i=1,10 do result[i]=(id-1)*10+i+offset end
                        end
                        return result
                    end
                    if key=='npcDrops' then return {9,2,1,2} end
                    if key=='vendors' then return {2,11} end
                    if key=='objectDrops' then return {5} end
                    return {999}
                end}}
            for _,name in ipairs({'Npc','Object'}) do
                local entityName=name
                LibQuestieDB[name]={Exists=function(id)
                    calls[entityName][id]=(calls[entityName][id] or 0)+1
                    if mode=='exists_throw' then error('PRIVATE') end
                    if mode=='exists_secret' then return secret end
                    if mode=='exists_change' then return calls[entityName][id]%2==1 end
                    return mode~='not_in_provider'
                end,Get=function(id,key)
                    assert(key=='spawns');spawnCalls=spawnCalls+1
                    if mode=='getter_throw' then error('PRIVATE') end
                    if mode=='getter_secret' then return secret end
                    if mode=='nil' then return nil end
                    if mode=='empty' then return {} end
                    if mode=='changing' then return {[1]={{spawnCalls,2}}} end
                    if mode=='equivalent' then
                        return spawnCalls%2==0 and {[1]={{1,2},{1,2,0}}}
                            or {[1]={{1,2,0},{1,2}}}
                    end
                    if mode=='pointer_change' then LibQuestieDB={} end
                    if mode=='points16' or mode=='points4096' or mode=='points4097' then
                        local n=mode=='points16' and 17 or mode=='points4096' and 4096 or 4097
                        local t={};for i=1,n do t[i]={1,2} end;return {[1]=t}
                    end
                    return input
                end}
            end
            if mode=='missing' then LibQuestieDB.Npc=nil;LibQuestieDB.Object=nil end
            if mode=='one_missing' then LibQuestieDB.Object=nil end
            if mode=='namespace_throw' then
                LibQuestieDB.Npc=nil
                setmetatable(LibQuestieDB,{__index=function(_,key)
                    assert(key=='Npc');error('PRIVATE') end})
            end
            if mode=='namespace_secret' then LibQuestieDB.Npc=secret end
            if mode=='lookup_throw' then LibQuestieDB.Npc=setmetatable({},{__index=function() error('PRIVATE') end}) end
            if mode=='lookup_secret' then LibQuestieDB.Npc={Exists=secret,Get=secret} end
            if badInput then input=badInput end
        end
        "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(
        r#"
        AO.collectors={recipe_item_sources=AO.collectors.recipe_item_sources}
        AO.collectors.profession_recipe_materials=function()
            local reagents={};local extra={}
            if mode=='budget' then
                for i=1,64 do reagents[tostring(i)]=1 end
                extra['65']=1
            else reagents['1']=1 end
            return {coverage='reported_known_spell_schematics',profession_id=164,
                entries={['101']={status='fixed_items',reagents=reagents},
                    ['102']={status='fixed_items',reagents=extra}}}
        end
        "#,
    )
    .exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua, mode: &str) -> Result<Value, Box<dyn std::error::Error>> {
    lua.globals().set("mode", mode)?;
    lua.load("setup(); AO.Capture()").exec()?;
    lua.load("assert(forbiddenCalls==0)").exec()?;
    let text: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(!text.contains("PRIVATE"));
    let snapshot = decode(format!("AzerothOperatorExport=\"{text}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    Ok(snapshot
        .sections
        .get("recipe_item_sources")
        .cloned()
        .unwrap_or(Value::Null))
}

#[test]
fn public_provider_areas_round_trip_without_reinterpreting_or_mutating_points()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let section = capture(&lua, "normal")?;
    let locations = &section["entity_locations"];
    assert_eq!(locations["coordinate_system"], "questiedb_area_percent");
    assert_eq!(locations["reported_npc_count"], 4);
    assert_eq!(locations["reported_object_count"], 1);
    assert_eq!(locations["npcs"].as_object().unwrap().len(), 4);
    assert_eq!(locations["objects"].as_object().unwrap().len(), 1);
    let spawns = &locations["npcs"]["1"]["spawns"];
    assert_eq!(spawns["reported_point_count"], 6);
    assert_eq!(spawns["truncated"], false);
    assert_eq!(
        spawns["entries"]["1"],
        json!({"kind":"coordinate","area_id":2,"x_percent":1,"y_percent":2})
    );
    assert_eq!(
        spawns["entries"]["2"],
        json!({"kind":"coordinate","area_id":7,"x_percent":0,"y_percent":100})
    );
    assert_eq!(spawns["entries"]["3"], spawns["entries"]["4"]);
    assert!(spawns["entries"]["3"].get("provider_phase_id").is_none());
    assert_eq!(spawns["entries"]["5"]["provider_phase_id"], 2);
    assert_eq!(
        spawns["entries"]["6"],
        json!({"kind":"instance_presence","area_id":7})
    );
    lua.load("assert(input[7][1][3]==0 and input[7][2][1]==-1 and #input[7]==5); assert(calls.Npc[999]==nil and calls.Object[999]==nil)").exec()?;
    let legacy = capture(&lua, "missing")?;
    assert!(legacy.get("entity_locations").is_none());
    assert_eq!(legacy["entries"], section["entries"]);
    Ok(())
}

#[test]
fn optional_failures_preserve_item_references_and_known_existence()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let baseline = capture(&lua, "normal")?["entries"].clone();
    for mode in [
        "one_missing",
        "namespace_throw",
        "namespace_secret",
        "lookup_throw",
        "lookup_secret",
    ] {
        let section = capture(&lua, mode)?;
        assert_eq!(section["entries"], baseline, "{mode}");
        let (namespace, id) = if mode == "one_missing" {
            ("objects", "5")
        } else {
            ("npcs", "1")
        };
        assert_eq!(
            section["entity_locations"][namespace][id],
            json!({"status":"unavailable"}),
            "{mode}"
        );
    }
    for mode in ["getter_throw", "getter_secret", "nil"] {
        let section = capture(&lua, mode)?;
        assert_eq!(section["entries"], baseline);
        assert_eq!(
            section["entity_locations"]["npcs"]["1"],
            json!({"status":"reported"}),
            "{mode}"
        );
    }
    for mode in ["exists_throw", "exists_secret", "exists_change", "changing"] {
        assert_eq!(
            capture(&lua, mode)?["entity_locations"]["npcs"]["1"],
            json!({"status":"unavailable"}),
            "{mode}"
        );
    }
    assert_eq!(
        capture(&lua, "not_in_provider")?["entity_locations"]["npcs"]["1"],
        json!({"status":"not_in_provider"})
    );
    assert_eq!(
        capture(&lua, "empty")?["entity_locations"]["npcs"]["1"]["spawns"],
        json!({"reported_point_count":0,"truncated":false,"entries":{}})
    );
    assert_eq!(
        capture(&lua, "equivalent")?["entity_locations"]["npcs"]["1"]["spawns"]["reported_point_count"],
        2
    );
    for mode in ["context_change", "pointer_change"] {
        assert!(capture(&lua, mode)?.is_null(), "{mode}");
    }
    Ok(())
}

#[test]
fn caches_only_retained_references_with_independent_entity_and_point_budgets()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let section = capture(&lua, "budget")?;
    let locations = &section["entity_locations"];
    assert_eq!(section["reported_item_count"], 65);
    assert_eq!(locations["reported_npc_count"], 1024);
    assert_eq!(locations["reported_object_count"], 512);
    for name in ["npcs", "objects"] {
        let rows = locations[name].as_object().unwrap();
        assert_eq!(rows.len(), 32);
        assert!(rows.contains_key("38"));
        assert!(!rows.contains_key("39"));
        assert!(!rows.contains_key("9"));
    }
    lua.load("for _,rows in pairs(calls) do local n=0;for id,count in pairs(rows) do n=n+1;assert(id<=38 and count==2) end;assert(n==32) end").exec()?;
    for mode in ["item_change", "item_unknown"] {
        let section = capture(&lua, mode)?;
        assert_eq!(
            section["entity_locations"],
            json!({"coordinate_system":"questiedb_area_percent", "reported_npc_count":0,
                "reported_object_count":0, "npcs":{}, "objects":{}}),
            "{mode}"
        );
        lua.load("assert(next(calls.Npc)==nil and next(calls.Object)==nil)")
            .exec()?;
    }
    for (mode, count) in [("points16", 17), ("points4096", 4096)] {
        let section = capture(&lua, mode)?;
        let spawns = &section["entity_locations"]["npcs"]["1"]["spawns"];
        assert_eq!(spawns["reported_point_count"], count);
        assert_eq!(spawns["truncated"], true);
        assert_eq!(spawns["entries"].as_object().unwrap().len(), 16);
        assert_eq!(spawns["entries"]["1"], spawns["entries"]["16"]);
    }
    assert_eq!(
        capture(&lua, "points4097")?["entity_locations"]["npcs"]["1"],
        json!({"status":"reported"})
    );
    Ok(())
}

#[test]
fn rejects_malformed_secret_sparse_and_out_of_range_spawn_tables()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for expression in [
        "(function() local areas={} for i=1,4097 do areas[i]={} end return areas end)()",
        "secret",
        "{[1]=secret}",
        "{[1]={secret}}",
        "{[1]={{secret,2}}}",
        "{[1]={{1,2,secret}}}",
        "{[secret]={{1,2}}}",
        "{[1]={[secret]={1,2}}}",
        "{[1]={{[secret]=1,[1]=1,[2]=2}}}",
        "{['1']={{1,2}}}",
        "{[0]={{1,2}}}",
        "{[4294967296]={{1,2}}}",
        "{[1.5]={{1,2}}}",
        "{[1]={[2]={1,2}}}",
        "{[1]={[0]={1,2}}}",
        "{[1]={[4097]={1,2}}}",
        "{[1]={['1']={1,2}}}",
        "{[1]={{1}}}",
        "{[1]={{1,2,3,4}}}",
        "{[1]={{[1]=1,[3]=3}}}",
        "{[1]={{1,2,name='PRIVATE'}}}",
        "{[1]={{-1,2}}}",
        "{[1]={{-1,-1,0}}}",
        "{[1]={{100.01,2}}}",
        "{[1]={{1,101}}}",
        "{[1]={{0/0,2}}}",
        "{[1]={{math.huge,2}}}",
        "{[1]={{1,2,-1}}}",
        "{[1]={{1,2,1.5}}}",
        "{[1]={{1,2,4294967296}}}",
        "{[1]={{1,'2'}}}",
    ] {
        lua.load(format!("badInput={expression}")).exec()?;
        let section = capture(&lua, "normal")?;
        assert_eq!(
            section["entity_locations"]["npcs"]["1"],
            json!({"status":"reported"}),
            "{expression}"
        );
        assert!(section["entries"]["1"].get("npc_drops").is_some());
    }
    lua.load("badInput={[4294967295]={{0.5,100,4294967295}}}")
        .exec()?;
    let section = capture(&lua, "normal")?;
    assert_eq!(
        section["entity_locations"]["npcs"]["1"]["spawns"]["entries"]["1"],
        json!({"kind":"coordinate","area_id":4294967295_u32,"x_percent":0.5,"y_percent":100,"provider_phase_id":4294967295_u32})
    );
    Ok(())
}
