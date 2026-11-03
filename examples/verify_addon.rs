//! Execute an actual staged addon in a Lua 5.1 fixture; never a live compatibility claim.
use azeroth_operator::import::decode;
use mlua::Lua;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "Usage: verify_addon <directory> <family> <variant> <interface>"
    );
    let interface: u32 = args[3].parse()?;
    let lua = Lua::new();
    lua.globals()
        .set("expected_interface", interface)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let fixture = r#"
        AO={}; SlashCmdList={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return "fixture", "1", "", expected_interface end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function UnitLevel() return 1 end
        function UnitClass() return "Fixture", "WARRIOR" end
        function UnitRace() return "Fixture", "Human" end
        function UnitName() return "PackageFixture" end
        function GetRealmName() return "Synthetic" end
        function GetServerTime() return 1735689600 end
        function date() return "2025-01-01T00:00:00Z" end
    "#;
    let result = (|| -> mlua::Result<String> {
        lua.load(fixture).exec()?;
        let namespace: mlua::Table = lua.globals().get("AO")?;
        for name in [
            "Adapter.lua",
            "Collectors.lua",
            "Core.lua",
            "MacroOperations.lua",
            "BindingOperations.lua",
            "BindingImport.lua",
            "EquipmentOperations.lua",
            "EquipmentImport.lua",
            "BagOperations.lua",
            "BagImport.lua",
            "MacroImport.lua",
            "MacroUI.lua",
        ] {
            let source = std::fs::read_to_string(Path::new(&args[0]).join(name))
                .map_err(mlua::Error::external)?;
            lua.load(&source)
                .call::<()>(("AzerothOperator", namespace.clone()))?;
        }
        lua.load("AO.Capture()").exec()?;
        let previous: String = lua.globals().get("AzerothOperatorExport")?;
        lua.load("expected_interface=expected_interface+1; assert(not pcall(AO.Capture))")
            .exec()?;
        assert_eq!(
            previous,
            lua.globals().get::<String>("AzerothOperatorExport")?
        );
        Ok(previous)
    })()
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{result}\"").as_bytes())?;
    anyhow::ensure!(
        serde_json::to_value(&snapshot.client.family)? == args[1],
        "Family mismatch"
    );
    anyhow::ensure!(
        snapshot.client.variant == args[2] && snapshot.client.interface == interface,
        "Client mismatch"
    );
    anyhow::ensure!(
        snapshot.sections["equipment"]["16"]["item_id"] == 123,
        "Item observation mismatch"
    );
    println!(
        "{}",
        serde_json::json!({"family":args[1],"variant":args[2],"interface":interface,
        "packaged_lua51_roundtrip":true,"changed_interface_rejected":true,"live_qualified":false})
    );
    Ok(())
}
