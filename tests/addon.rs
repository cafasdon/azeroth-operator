use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn lua_export_roundtrip_and_failed_capture_preserves_previous()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO = {}; SlashCmdList = {}; combat = false
        function InCombatLockdown() return combat end
        function GetBuildInfo() return "12.1.5", "70688", "", 120105 end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink(_,slot) if slot==16 then return 'quoted "item" \\ link' end end
        function UnitLevel() return 80 end
        function UnitClass() return "Shaman", "SHAMAN" end
        function UnitRace() return "Goblin", "Goblin" end
        function UnitName() return "Example" end
        function GetRealmName() return "Test" end
        function GetServerTime() return 1735689600 end
        function date() return "2025-01-01T00:00:00Z" end
        C_Container = {}
        function C_Container.GetContainerNumSlots() return 1 end
        function C_Container.GetContainerItemInfo(bag,slot)
            if bag==0 then return {itemID=456,stackCount=2,hyperlink="test"} end
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../clients/retail/Adapter.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    assert_eq!(snapshot.sections["bags"]["0"]["1"]["count"], 2);
    assert_eq!(
        snapshot.sections["equipment"]["16"]["link"],
        "quoted \"item\" \\ link"
    );
    lua.load("combat=true; assert(not pcall(AO.Capture))")
        .exec()?;
    assert_eq!(hex, lua.globals().get::<String>("AzerothOperatorExport")?);
    Ok(())
}
