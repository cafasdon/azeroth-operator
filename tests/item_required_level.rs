use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn required_level_is_optional_linked_and_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';NUM_TOTAL_EQUIPPED_BAG_SLOTS=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE' or (mode=='secret' and v==30) end
        function GetInventoryItemID(_,slot) if slot==10 then return 720 end end
        function GetInventoryItemLink() if mode~='no_link' then return 'item:720:0:0:0:0:0:0' end end
        C_Container={GetContainerNumSlots=function() return 1 end,
            GetContainerItemInfo=function() return {itemID=720,stackCount=1,hyperlink=GetInventoryItemLink()} end}
        C_Item={GetItemInfoInstant=function() return mode=='wrong_id' and 721 or 720 end,
            GetItemInfo=function(link)
                assert(link=='item:720:0:0:0:0:0:0','wrong native query')
                if mode=='throw' then error('PRIVATE') end
                if mode=='uncached' then return nil end
                local level=30
                if mode=='zero' then level=0 end
                if mode=='bad' then level=true end
                return 'PRIVATE','PRIVATE','PRIVATE','PRIVATE',level
            end,
            RequestLoadItemDataByID=function() error('load forbidden') end,
            EquipItemByName=function() error('equip forbidden') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("AO.Capture()").exec()?;
        let encoded: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{encoded}\"").as_bytes(),
        )?)
    };
    for (mode, expected) in [
        ("normal", Some(30)),
        ("zero", Some(0)),
        ("secret", None),
        ("bad", None),
        ("uncached", None),
        ("throw", None),
        ("wrong_id", None),
        ("no_link", None),
    ] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert_eq!(
            saved.sections["equipment"]["10"]["required_level"].as_u64(),
            expected,
            "{mode}"
        );
        assert_eq!(
            saved.sections["bags"]["0"]["1"]["required_level"].as_u64(),
            expected,
            "{mode}"
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("mode='normal';AO.family='retail'").exec()?;
    assert!(capture()?.sections["equipment"]["10"]["required_level"].is_null());
    Ok(())
}
