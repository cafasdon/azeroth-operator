use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn native_item_stats_are_link_bound_optional_and_stable() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0;NUM_TOTAL_EQUIPPED_BAG_SLOTS=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE' end
        function GetInventoryItemID(_,slot) if slot==16 then return 720 end end
        function GetInventoryItemLink() if mode~='no_link' then return 'item:720:0:0:0:0:0:0' end end
        C_Container={GetContainerNumSlots=function() return 1 end,GetContainerItemInfo=function()
            return {itemID=720,stackCount=1,hyperlink=GetInventoryItemLink()} end}
        C_Item={GetItemInfoInstant=function() return mode=='wrong_id' and 721 or 720 end,
            GetItemStats=function(link)
                assert(link=='item:720:0:0:0:0:0:0');reads=reads+1
                if mode=='error' then error('PRIVATE') end
                if mode=='restricted_table' then return 'PRIVATE' end
                if mode=='uncached' then return nil end
                if mode=='empty' then return {} end
                if mode=='oversize' then local t={};for i=1,129 do t['ITEM_MOD_STAT_'..i..'_SHORT']=1 end;return t end
                local result={ITEM_MOD_STRENGTH_SHORT=-2,ITEM_MOD_DAMAGE_PER_SECOND_SHORT=8.86,RESISTANCE0_NAME=25,EMPTY_SOCKET_RED=0}
                if mode=='restricted_value' then result.RESISTANCE0_NAME='PRIVATE' end
                if mode=='restricted_key' then result.PRIVATE=1 end
                if mode=='bad_key' then result.crafterName=1 end
                if mode=='nan' then result.RESISTANCE0_NAME=0/0 end
                if mode=='infinity' then result.RESISTANCE0_NAME=math.huge end
                if mode=='overflow' then result.RESISTANCE0_NAME=4294967296 end
                if mode=='boolean' then result.RESISTANCE0_NAME=true end
                if mode=='changed' and reads%2==0 then result.RESISTANCE0_NAME=26 end
                if mode=='added' and reads%2==0 then result.ITEM_MOD_AGILITY_SHORT=1 end
                return result
            end,
            RequestLoadItemDataByID=function() error('load forbidden') end,
            EquipItemByName=function() error('equip forbidden') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads=0;AO.Capture()").exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    let saved = capture("normal")?;
    let stats = &saved.sections["equipment"]["16"]["native_stats"];
    assert_eq!(stats["ITEM_MOD_STRENGTH_SHORT"], -2);
    assert_eq!(stats["ITEM_MOD_DAMAGE_PER_SECOND_SHORT"], 8.86);
    assert_eq!(saved.sections["bags"]["0"]["1"]["native_stats"], *stats);
    assert_eq!(
        capture("empty")?.sections["equipment"]["16"]["native_stats"],
        serde_json::json!({})
    );
    for mode in [
        "no_link",
        "wrong_id",
        "error",
        "restricted_table",
        "uncached",
        "oversize",
        "restricted_value",
        "restricted_key",
        "bad_key",
        "nan",
        "infinity",
        "overflow",
        "boolean",
        "changed",
        "added",
    ] {
        let saved = capture(mode)?;
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 720);
        assert!(
            saved.sections["equipment"]["16"]
                .get("native_stats")
                .is_none(),
            "{mode}"
        );
        assert!(
            saved.sections["bags"]["0"]["1"]
                .get("native_stats")
                .is_none(),
            "{mode}"
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("C_Item.GetItemStats=nil").exec()?;
    assert!(
        capture("normal")?.sections["equipment"]["16"]
            .get("native_stats")
            .is_none()
    );
    Ok(())
}
