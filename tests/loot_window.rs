use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'
        counts=0; visibility=0; links={}; infos={}; actions=0; trace={}
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() end
        SECRET=setmetatable({}, {__tostring=function() error('secret inspected') end})
        function issecretvalue(v) return v==SECRET or v=='PRIVATE' end
        LootFrame={IsShown=function()
            visibility=visibility+1
            if mode=='secret_visibility' then return SECRET end
            if mode=='bad_visibility' then return 1 end
            if mode=='visibility_error' then error('PRIVATE') end
            return mode~='hidden' and not(mode=='closed' and visibility>1)
                and not(mode=='closed_second_read' and visibility==4)
                and not(mode=='closed_between' and visibility==3)
        end}
        function GetNumLootItems()
            counts=counts+1
            if mode=='empty' then return 0 end
            if mode=='boundary' then return 64 end
            if mode=='secret_count' then return SECRET end
            if mode=='oversize' then return 65 end
            if mode=='negative_count' then return -1 end
            if mode=='float_count' then return 1.5 end
            if mode=='nan_count' then return 0/0 end
            if mode=='count_error' then error('PRIVATE') end
            if mode=='count_changed' and counts==2 then return 1 end
            if mode=='count_changed_between' and counts==3 then return 1 end
            if mode=='count_changed_final' and counts==4 then return 1 end
            return 2
        end
        function GetLootSlotLink(slot)
            links[slot]=(links[slot] or 0)+1
            trace[#trace+1]=slot
            if slot==1 then
                if mode=='secret_link' then return SECRET end
                if mode=='link_error' or mode=='second_link_error' and links[slot]>1 then error('PRIVATE') end
                if mode=='nil_link' then return nil end
                if mode=='number_link' then return 123 end
                if mode=='long_link' then return '|Hitem:1:|h'..string.rep('x',2048)..'|h' end
                if mode=='exact_link_limit' then return '|Hitem:1:|h'..string.rep('x',2035)..'|h' end
                if mode=='currency_link' then return '|Hcurrency:1:|h[PRIVATE]|h' end
                if mode=='money' then return 'PRIVATE money' end
                if mode=='missing_suffix' then return '|Hitem:1:' end
                if mode=='prefix_junk' then return 'junk|Hitem:1:|h[PRIVATE]|h' end
                if mode=='suffix_junk' then return '|Hitem:1:|h[PRIVATE]|hjunk' end
                if mode=='zero_id' then return '|Hitem:0:|h[PRIVATE]|h' end
                if mode=='leading_zero_id' then return '|Hitem:01:|h[PRIVATE]|h' end
                if mode=='negative_id' then return '|Hitem:-1:|h[PRIVATE]|h' end
                if mode=='float_id' then return '|Hitem:1.5:|h[PRIVATE]|h' end
                if mode=='exponent_id' then return '|Hitem:1e3:|h[PRIVATE]|h' end
                if mode=='large_id' then return '|Hitem:4294967296:|h[PRIVATE]|h' end
                if mode=='huge_id' then return '|Hitem:'..string.rep('9',100)..':|h[PRIVATE]|h' end
                if mode=='changed_id' and links[slot]>1 then return '|Hitem:2:|h[PRIVATE]|h' end
                if mode=='boundary' then return '|Hitem:4294967295:|h[PRIVATE]|h' end
            end
            return '|cffffffff|Hitem:'..(slot==1 and 4549 or 118)..':0:0|h[PRIVATE]|h|r'
        end
        function GetLootSlotInfo(slot)
            infos[slot]=(infos[slot] or 0)+1
            local quantity=slot==1 and 3 or 0
            if slot==1 then
                if mode=='info_error' or mode=='second_info_error' and infos[slot]>1 then error('PRIVATE') end
                if mode=='secret_quantity' then quantity=SECRET end
                if mode=='nil_quantity' then quantity=nil end
                if mode=='string_quantity' then quantity='3' end
                if mode=='negative_quantity' then quantity=-1 end
                if mode=='float_quantity' then quantity=1.5 end
                if mode=='nan_quantity' then quantity=0/0 end
                if mode=='infinite_quantity' then quantity=math.huge end
                if mode=='large_quantity' then quantity=4294967296 end
                if mode=='changed_quantity' and infos[slot]>1 then quantity=4 end
                if mode=='boundary' then quantity=4294967295 end
            end
            -- Only the third return is observed. Names, texture and extra data are restricted.
            return SECRET,SECRET,quantity,SECRET,SECRET,SECRET,SECRET
        end
        function forbidden() actions=actions+1; error('unexpected action') end
        LootSlot=forbidden; LootSlotItem=forbidden; ConfirmLootSlot=forbidden
        CloseLoot=forbidden; ClearLootSlot=forbidden; RollOnLoot=forbidden
        ConfirmLootRoll=forbidden; LootFrame.Show=forbidden; LootFrame.Hide=forbidden
        LootFrame.RegisterEvent=forbidden; LootFrame.SetScript=forbidden
        hooksecurefunc=forbidden; GetLootSourceInfo=forbidden; GetLootSlotType=forbidden
        function capture()
            counts=0;visibility=0;links={};infos={};trace={}
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
    let value: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{value}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok(snapshot)
}

#[test]
fn loot_rows_are_two_pass_public_observations() -> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    let snapshot = capture(&lua, "normal")?;
    assert_eq!(
        snapshot.sections["loot_window"],
        json!({
            "coverage":"visible_loot_window_slots", "slot_count":2,
            "entries":{"1":{"status":"item","item_id":4549,"quantity":3},
                       "2":{"status":"item","item_id":118,"quantity":0}}
        })
    );
    lua.load("assert(table.concat(trace,',')=='1,2,1,2'); assert(counts==4 and visibility==4)")
        .exec()?;
    let empty = capture(&lua, "empty")?;
    assert_eq!(empty.sections["loot_window"]["slot_count"], 0);
    assert_eq!(empty.sections["loot_window"]["entries"], json!({}));
    lua.load("assert(next(links)==nil and next(infos)==nil)")
        .exec()?;
    let boundary = capture(&lua, "boundary")?;
    assert_eq!(boundary.sections["loot_window"]["slot_count"], 64);
    assert_eq!(
        boundary.sections["loot_window"]["entries"]
            .as_object()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        boundary.sections["loot_window"]["entries"]["1"],
        json!({
            "status":"item", "item_id":4294967295_u32, "quantity":4294967295_u32
        })
    );
    assert_eq!(
        capture(&lua, "exact_link_limit")?.sections["loot_window"]["entries"]["1"]["status"],
        "item"
    );
    // Exercise the actual manual command and decode its SavedVariables payload.
    lua.load("mode='normal'; SlashCmdList.AZEROTHOPERATOR('')")
        .exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    assert!(
        decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?
            .sections
            .contains_key("loot_window")
    );
    Ok(())
}

#[test]
fn unreadable_or_unstable_rows_preserve_independent_items() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = fixture()?;
    for mode in [
        "secret_link",
        "link_error",
        "second_link_error",
        "nil_link",
        "number_link",
        "long_link",
        "currency_link",
        "money",
        "missing_suffix",
        "prefix_junk",
        "suffix_junk",
        "zero_id",
        "leading_zero_id",
        "negative_id",
        "float_id",
        "exponent_id",
        "large_id",
        "huge_id",
        "changed_id",
        "info_error",
        "second_info_error",
        "secret_quantity",
        "nil_quantity",
        "string_quantity",
        "negative_quantity",
        "float_quantity",
        "nan_quantity",
        "infinite_quantity",
        "large_quantity",
        "changed_quantity",
    ] {
        let snapshot = capture(&lua, mode)?;
        assert_eq!(
            snapshot.sections["loot_window"]["entries"]["1"],
            json!({"status":"unreadable_or_non_item"}),
            "{mode}"
        );
        assert_eq!(
            snapshot.sections["loot_window"]["entries"]["2"],
            json!({"status":"item","item_id":118,"quantity":0}),
            "{mode}"
        );
        assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    }
    Ok(())
}

#[test]
fn unavailable_context_never_claims_loot_coverage() -> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for mode in [
        "hidden",
        "closed",
        "closed_second_read",
        "closed_between",
        "secret_visibility",
        "bad_visibility",
        "visibility_error",
        "secret_count",
        "oversize",
        "negative_count",
        "float_count",
        "nan_count",
        "count_error",
        "count_changed",
        "count_changed_between",
        "count_changed_final",
    ] {
        let snapshot = capture(&lua, mode)?;
        assert!(!snapshot.sections.contains_key("loot_window"), "{mode}");
        assert_eq!(
            snapshot.unavailable["loot_window"], "api_unavailable_or_failed",
            "{mode}"
        );
        assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    }
    for api in [
        "GetNumLootItems",
        "GetLootSlotLink",
        "GetLootSlotInfo",
        "LootFrame.IsShown",
        "LootFrame",
    ] {
        lua.load(format!("saved={api}; {api}=nil")).exec()?;
        assert!(
            !capture(&lua, "normal")?
                .sections
                .contains_key("loot_window"),
            "{api}"
        );
        assert!(
            !capture(&lua, "empty")?.sections.contains_key("loot_window"),
            "{api}"
        );
        lua.load(format!("{api}=saved")).exec()?;
    }
    lua.load("AO.family='retail'; assert(not pcall(AO.collectors.loot_window))")
        .exec()?;
    let retail = capture(&lua, "normal")?;
    assert!(!retail.sections.contains_key("loot_window"));
    assert!(!retail.unavailable.contains_key("loot_window"));
    lua.load("AO.family='forever'").exec()?;
    assert!(capture(&lua, "combat").is_err());
    lua.load("assert(counts==0 and next(links)==nil)").exec()?;
    Ok(())
}
