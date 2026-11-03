//! Fixed synthetic Lua capture benchmark; no game process or account data.
use mlua::Lua;
use std::time::Instant;

fn run(core: &str) -> Result<(f64, String), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='retail',variant='live'}; SlashCmdList={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '12.1.5','70688','',120105 end
        function UnitName() return 'Benchmark' end
        function GetRealmName() return 'Synthetic' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function UnitLevel() return 80 end
        function UnitClass() return 'Shaman','SHAMAN' end
        function UnitRace() return 'Goblin','Goblin' end
        function GetInventoryItemID(_,slot) return 1000+slot end
        function GetInventoryItemLink(_,slot) return 'item:'..(1000+slot)..':"quoted"' end
        NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        C_Container={GetContainerNumSlots=function() return 30 end,
            GetContainerItemInfo=function(bag,slot)
                return {itemID=2000+bag*30+slot,stackCount=2,
                    hyperlink='item:'..(2000+bag*30+slot)..':"quoted"'}
            end}
    "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(core).call::<()>(("AzerothOperator", namespace))?;
    lua.load("for i=1,10 do AO.Capture() end").exec()?;
    let start = Instant::now();
    lua.load("for i=1,100 do AO.Capture() end").exec()?;
    let elapsed = start.elapsed().as_secs_f64() * 10.0;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    azeroth_operator::import::decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    Ok((elapsed, hex))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "benchmark_addon <before-core> <after-core>");
    let before = std::fs::read_to_string(&args[0])?;
    let after = std::fs::read_to_string(&args[1])?;
    let mut pairs = Vec::new();
    for index in 0..4 {
        let (old, new) = if index % 2 == 0 {
            (run(&before)?, run(&after)?)
        } else {
            let new = run(&after)?;
            (run(&before)?, new)
        };
        assert_eq!(old.1, new.1, "Export bytes changed");
        pairs.push(serde_json::json!({"before_ms":old.0,"after_ms":new.0}));
    }
    println!(
        "{}",
        serde_json::json!({"pairs":pairs,"captures_per_run":100,
        "warmup_captures":10,"bag_items":180,"equipment_items":19,
        "export_bytes_identical":true,"live_game_measurement":false})
    );
    Ok(())
}
