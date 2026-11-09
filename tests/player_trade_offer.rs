use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'
        visibility=0; calls={}; trace={}; actions=0
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player'); return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() end
        SECRET=setmetatable({}, {__tostring=function() error('secret inspected') end})
        UNUSED=setmetatable({}, {__tostring=function() error('unused inspected') end})
        function issecretvalue(value)
            if rawequal(value,UNUSED) then actions=actions+1; error('unused tuple inspected') end
            return rawequal(value,SECRET) or value=='PRIVATE'
        end
        function forbidden() actions=actions+1; error('unexpected action or forbidden getter') end
        function mutate()
            if mutation=='frame' then TradeFrame={IsShown=function() return true end} end
            if mutation=='shown' then TradeFrame.IsShown=function() return true end end
            if mutation=='getter' then GetTradePlayerItemInfo=function() return nil end end
            if mutation=='family' then AO.family='retail' end
            if mutation=='guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
        end
        TradeFrame={IsShown=function()
            visibility=visibility+1
            if mode=='mutate_visibility' and visibility==4 then mutate() end
            if mode=='secret_visibility' then return SECRET end
            if mode=='bad_visibility' then return 1 end
            if mode=='nil_visibility' then return nil end
            if mode=='visibility_error' then error('PRIVATE') end
            return mode~='hidden' and not(mode=='closed' and visibility==2)
                and not(mode=='closed_between' and visibility==3)
                and not(mode=='closed_final' and visibility==4)
        end}
        function GetTradePlayerItemInfo(slot)
            assert(slot>=1 and slot<=6, 'non-normal trade slot read')
            calls[slot]=(calls[slot] or 0)+1; trace[#trace+1]=slot
            if mode=='mutate_info' and slot==6 and calls[slot]==2 then mutate() end
            local itemID,quantity=100+slot,slot
            if mode=='empty' then return nil end
            if slot==1 then
                if mode=='error' or mode=='second_error' and calls[slot]==2 then error('PRIVATE') end
                if mode=='empty_first' and calls[slot]==1 then return nil end
                if mode=='empty_second' and calls[slot]==2 then return nil end
                if mode=='bad_id' then itemID=badValue end
                if mode=='bad_quantity' then quantity=badValue end
                if mode=='changed_id' and calls[slot]==2 then itemID=999 end
                if mode=='changed_quantity' and calls[slot]==2 then quantity=999 end
                if mode=='boundary' then itemID=4294967295; quantity=4294967295 end
            end
            -- Every unused return is inaccessible, including name and enchantment.
            return UNUSED,UNUSED,quantity,UNUSED,UNUSED,UNUSED,UNUSED,itemID,UNUSED
        end
        GetTradeTargetItemInfo=forbidden; GetTradeTargetItemLink=forbidden
        GetTradePlayerItemLink=forbidden; GetPlayerTradeMoney=forbidden
        GetTargetTradeMoney=forbidden; GetTradeTargetName=forbidden
        UnitGUID=forbidden; AcceptTrade=forbidden; CancelTrade=forbidden
        InitiateTrade=forbidden; ClickTradeButton=forbidden; SetTradeMoney=forbidden
        PickupPlayerMoney=forbidden; ClearCursor=forbidden; PickupContainerItem=forbidden
        TradeFrame.Show=forbidden; TradeFrame.Hide=forbidden
        TradeFrame.RegisterEvent=forbidden; TradeFrame.SetScript=forbidden
        hooksecurefunc=forbidden
        function capture()
            visibility=0;calls={};trace={}
            AO.Capture()
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua, mode: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
    lua.globals().set("mode", mode)?;
    lua.load("capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok(snapshot)
}

#[test]
fn six_normal_player_slots_are_read_in_two_complete_passes()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let snapshot = capture(&lua, "normal")?;
    let section = &snapshot.sections["player_trade_offer"];
    assert_eq!(section["coverage"], "visible_player_trade_normal_slots");
    assert_eq!(section.as_object().unwrap().len(), 2);
    assert_eq!(section["entries"].as_object().unwrap().len(), 6);
    for slot in 1..=6 {
        assert_eq!(
            section["entries"][slot.to_string()],
            json!({
                "status":"item", "item_id":100+slot, "quantity":slot
            })
        );
    }
    lua.load("assert(table.concat(trace,',')=='1,2,3,4,5,6,1,2,3,4,5,6'); assert(visibility==4)")
        .exec()?;
    let boundary = capture(&lua, "boundary")?;
    assert_eq!(
        boundary.sections["player_trade_offer"]["entries"]["1"],
        json!({
            "status":"item", "item_id":4294967295_u32, "quantity":4294967295_u32
        })
    );
    let empty = capture(&lua, "empty")?;
    assert_eq!(
        empty.sections["player_trade_offer"]["entries"]
            .as_object()
            .unwrap()
            .len(),
        6
    );
    for slot in 1..=6 {
        assert_eq!(
            empty.sections["player_trade_offer"]["entries"][slot.to_string()],
            json!({"status":"unreadable_or_empty"})
        );
    }
    lua.load("mode='normal'; SlashCmdList.AZEROTHOPERATOR('')")
        .exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(
        decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?
            .sections
            .contains_key("player_trade_offer")
    );
    Ok(())
}

#[test]
fn invalid_or_unstable_numbers_are_unknown_without_dropping_other_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for mode in ["bad_id", "bad_quantity"] {
        for value in [
            "nil",
            "SECRET",
            "'PRIVATE'",
            "'123'",
            "false",
            "{}",
            "0",
            "-1",
            "1.5",
            "0/0",
            "math.huge",
            "-math.huge",
            "4294967296",
        ] {
            lua.load(format!("badValue={value}")).exec()?;
            let snapshot = capture(&lua, mode)?;
            assert_eq!(
                snapshot.sections["player_trade_offer"]["entries"]["1"],
                json!({"status":"unreadable_or_empty"}),
                "{mode}: {value}"
            );
            assert_eq!(
                snapshot.sections["player_trade_offer"]["entries"]["6"],
                json!({"status":"item","item_id":106,"quantity":6})
            );
            lua.load("assert(table.concat(trace,',')=='1,2,3,4,5,6,1,2,3,4,5,6')")
                .exec()?;
        }
    }
    for mode in [
        "error",
        "second_error",
        "empty_first",
        "empty_second",
        "changed_id",
        "changed_quantity",
    ] {
        let snapshot = capture(&lua, mode)?;
        assert_eq!(
            snapshot.sections["player_trade_offer"]["entries"]["1"],
            json!({"status":"unreadable_or_empty"}),
            "{mode}"
        );
        assert_eq!(
            snapshot.sections["player_trade_offer"]["entries"]["2"],
            json!({"status":"item","item_id":102,"quantity":2})
        );
    }
    Ok(())
}

#[test]
fn absent_or_unreadable_context_never_claims_trade_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for mode in [
        "hidden",
        "closed",
        "closed_between",
        "closed_final",
        "secret_visibility",
        "bad_visibility",
        "nil_visibility",
        "visibility_error",
    ] {
        let snapshot = capture(&lua, mode)?;
        assert!(
            !snapshot.sections.contains_key("player_trade_offer"),
            "{mode}"
        );
        assert_eq!(
            snapshot.unavailable["player_trade_offer"],
            "api_unavailable_or_failed"
        );
        assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
        if mode == "hidden" {
            lua.load("assert(next(calls)==nil)").exec()?;
        }
    }
    for api in ["GetTradePlayerItemInfo", "TradeFrame.IsShown", "TradeFrame"] {
        for value in ["nil", "SECRET", "true", "123"] {
            let lua = fixture()?;
            lua.load(format!("{api}={value}")).exec()?;
            assert!(
                !capture(&lua, "normal")?
                    .sections
                    .contains_key("player_trade_offer"),
                "{api}={value}"
            );
            lua.load("assert(next(calls)==nil)").exec()?;
        }
    }
    lua.load("AO.family='retail'; assert(not pcall(AO.collectors.player_trade_offer))")
        .exec()?;
    let retail = capture(&lua, "normal")?;
    assert!(!retail.sections.contains_key("player_trade_offer"));
    assert!(!retail.unavailable.contains_key("player_trade_offer"));
    lua.load("assert(next(calls)==nil); AO.family='forever'")
        .exec()?;
    assert!(capture(&lua, "combat").is_err());
    lua.load("assert(next(calls)==nil)").exec()?;
    Ok(())
}

#[test]
fn final_callbacks_cannot_replace_the_observation_context() -> Result<(), Box<dyn std::error::Error>>
{
    for mode in ["mutate_visibility", "mutate_info"] {
        for mutation in ["frame", "shown", "getter", "family", "guard"] {
            let lua = fixture()?;
            lua.globals().set("mutation", mutation)?;
            let snapshot = capture(&lua, mode)?;
            assert!(
                !snapshot.sections.contains_key("player_trade_offer"),
                "{mode}: {mutation}"
            );
            assert_eq!(
                snapshot.unavailable["player_trade_offer"],
                "api_unavailable_or_failed"
            );
            assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
        }
    }
    Ok(())
}
