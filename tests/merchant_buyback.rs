use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn buyback_is_visible_stable_and_never_purchased() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';counts=0;ids={};infos={};visibility=0
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        MerchantFrame={selectedTab=2,IsVisible=function()
            visibility=visibility+1
            if mode=='secret_visibility' then return 'PRIVATE' end
            return mode~='hidden' and not(mode=='closed' and visibility>1)
        end}
        function GetNumBuybackItems()
            counts=counts+1
            if mode=='empty' then return 0 end
            if mode=='secret_count' then return 'PRIVATE' end
            if mode=='oversize' then return 13 end
            if mode=='count_changed' and counts>1 then return 1 end
            return 2
        end
        C_MerchantFrame={GetBuybackItemID=function(slot)
            ids[slot]=(ids[slot] or 0)+1
            if mode=='secret_id' then return 'PRIVATE' end
            if mode=='bad_id' then return 0 end
            if mode=='changed_id' and ids[slot]>1 then return 500 end
            return slot==1 and 4549 or 118
        end}
        function GetBuybackItemInfo(slot)
            infos[slot]=(infos[slot] or 0)+1
            if mode=='error' or mode=='second_error' and infos[slot]>1 then error('PRIVATE') end
            if mode=='tab_changed' then MerchantFrame.selectedTab=1 end
            local count=slot==1 and 3 or 1
            if mode=='secret_quantity' then count='PRIVATE' end
            if mode=='bad_quantity' then count=0 end
            if mode=='changed_quantity' and infos[slot]>1 then count=count+1 end
            local price=slot==1 and 10 or 0
            if slot==1 then
                if mode=='secret_price' then price='PRIVATE' end
                if mode=='nil_price' then price=nil end
                if mode=='bad_price' then price=-1 end
                if mode=='large_price' then price=10000000000000 end
                if mode=='float_price' then price=1.5 end
                if mode=='nan_price' then price=0/0 end
                if mode=='changed_price' and infos[slot]>1 then price=11 end
            end
            return 'PRIVATE','PRIVATE',price,count,'PRIVATE','PRIVATE','PRIVATE'
        end
        function BuybackItem() error('no buyback action') end
        function BuyMerchantItem() error('no purchase') end
        function UseContainerItem() error('no selling') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("counts=0;ids={};infos={};visibility=0;MerchantFrame.selectedTab=mode=='goods' and 1 or mode=='secret_tab' and 'PRIVATE' or 2;AO.Capture()").exec()?;
        let value: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{value}\"").as_bytes(),
        )?)
    };
    let initial = capture("normal")?;
    assert_eq!(
        initial.sections["merchant_buyback"]["entries"],
        serde_json::json!({
        "1":{"item_id":4549,"count":3,"buyback_price_copper":10},"2":{"item_id":118,"count":1,"buyback_price_copper":0}})
    );
    let summary = azeroth_operator::analysis::summary(&initial);
    assert_eq!(summary["merchant_buyback_observations"]["listed_units"], 4);
    assert_eq!(
        summary["merchant_buyback_observations"]["total_listed_quote_copper"],
        10
    );
    assert!(!serde_json::to_string(&initial)?.contains("PRIVATE"));
    for mode in [
        "secret_price",
        "nil_price",
        "bad_price",
        "large_price",
        "float_price",
        "nan_price",
        "changed_price",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["merchant_buyback"]["entries"]["1"]
                .get("buyback_price_copper")
                .is_none(),
            "{mode}"
        );
        let summary = azeroth_operator::analysis::summary(&value);
        assert_eq!(
            summary["merchant_buyback_observations"]["unknown_price_entries"],
            1
        );
        assert!(summary["merchant_buyback_observations"]["total_listed_quote_copper"].is_null());
    }
    for mode in [
        "hidden",
        "closed",
        "goods",
        "secret_tab",
        "secret_visibility",
        "secret_count",
        "oversize",
        "count_changed",
        "secret_id",
        "bad_id",
        "changed_id",
        "secret_quantity",
        "bad_quantity",
        "changed_quantity",
        "error",
        "second_error",
        "tab_changed",
    ] {
        let value = capture(mode)?;
        assert!(!value.sections.contains_key("merchant_buyback"), "{mode}");
        assert!(value.unavailable.contains_key("merchant_buyback"), "{mode}");
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
    }
    assert_eq!(
        capture("empty")?.sections["merchant_buyback"]["entries"],
        serde_json::json!({})
    );
    lua.load("C_MerchantFrame.GetBuybackItemID=nil;GetBuybackItemInfo=nil")
        .exec()?;
    assert!(!capture("normal")?.sections.contains_key("merchant_buyback"));
    assert!(capture("empty")?.sections.contains_key("merchant_buyback"));
    assert!(capture("combat").is_err());
    Ok(())
}
