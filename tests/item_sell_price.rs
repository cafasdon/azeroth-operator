use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn native_sell_prices_are_optional_stable_and_link_bound() -> Result<(), Box<dyn std::error::Error>>
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
            return {itemID=720,stackCount=3,hyperlink=GetInventoryItemLink()} end}
        C_Item={GetItemInfoInstant=function() return mode=='wrong_id' and 721 or mode=='secret_id' and 'PRIVATE' or 720 end,
            GetItemInfo=function(link)
                assert(link=='item:720:0:0:0:0:0:0');reads=reads+1
                if mode=='error' then error('PRIVATE') end
                if mode=='missing' then return end
                local price=100
                if mode=='zero' then price=0 end
                if mode=='maximum' then price=9999999999999 end
                if mode=='negative' then price=-1 end
                if mode=='overflow' then price=10000000000000 end
                if mode=='fraction' then price=0.5 end
                if mode=='boolean' then price=true end
                if mode=='nan' then price=0/0 end
                if mode=='infinity' then price=math.huge end
                if mode=='secret' then price='PRIVATE' end
                if mode=='changed' and reads%3==2 then price=101 end
                if mode=='second_secret' and reads%3==2 then price='PRIVATE' end
                if mode=='second_error' and reads%3==2 then error('PRIVATE') end
                return 'PRIVATE','PRIVATE','PRIVATE','PRIVATE',nil,'PRIVATE','PRIVATE','PRIVATE','PRIVATE','PRIVATE',price
            end,RequestLoadItemDataByID=function() error('load forbidden') end,
            SellItem=function() error('sale forbidden') end}
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
    for (mode, price) in [
        ("normal", 100u64),
        ("zero", 0),
        ("maximum", 9_999_999_999_999),
    ] {
        let saved = capture(mode)?;
        assert_eq!(
            saved.sections["equipment"]["16"]["sell_price_copper"],
            price
        );
        assert_eq!(saved.sections["bags"]["0"]["1"]["sell_price_copper"], price);
        assert_eq!(
            azeroth_operator::analysis::summary(&saved)["captured_vendor_price_values"]["bags"]["observed_total_copper"],
            price * 3
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in [
        "no_link",
        "wrong_id",
        "secret_id",
        "missing",
        "error",
        "negative",
        "overflow",
        "fraction",
        "boolean",
        "nan",
        "infinity",
        "secret",
        "changed",
        "second_secret",
        "second_error",
    ] {
        let saved = capture(mode)?;
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 720);
        assert!(
            saved.sections["equipment"]["16"]
                .get("sell_price_copper")
                .is_none(),
            "{mode}"
        );
        assert!(
            saved.sections["bags"]["0"]["1"]
                .get("sell_price_copper")
                .is_none(),
            "{mode}"
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("C_Item.GetItemInfo=nil").exec()?;
    assert!(
        capture("normal")?.sections["equipment"]["16"]
            .get("sell_price_copper")
            .is_none()
    );
    Ok(())
}
