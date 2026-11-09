use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn fixture(setup: &str) -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}
        actions=0; secretInspections=0; counts=0; visibility=0; typeCalls={}; infoCalls={}; linkCalls={}; trace={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() end
        SECRET=setmetatable({}, {__index=function() secretInspections=secretInspections+1; error('secret inspected') end,
            __tostring=function() secretInspections=secretInspections+1; error('secret inspected') end,
            __call=function() actions=actions+1; error('secret called') end})
        function issecretvalue(v) return rawequal(v,SECRET) end
        -- Arbitrary values deliberately prevent hard-coded native enum assumptions.
        Enum={LootSlotType={Item=17,Currency=23,Money=41}}
        kinds={23,41,17}; ids={81}; quantities={3,0,2}; size=3
        function forbidden() actions=actions+1; error('unexpected action') end
        LootFrame={IsShown=function()
            visibility=visibility+1
            if visibilityHook then return visibilityHook() end
            return true
        end,Show=forbidden,Hide=forbidden,RegisterEvent=forbidden,SetScript=forbidden}
        function GetNumLootItems()
            counts=counts+1
            if countHook then return countHook() end
            return size
        end
        function GetLootSlotType(slot)
            typeCalls[slot]=(typeCalls[slot] or 0)+1
            trace[#trace+1]=slot
            if typeHook then return typeHook(slot,typeCalls[slot]) end
            return kinds[slot]
        end
        function GetLootSlotInfo(slot)
            infoCalls[slot]=(infoCalls[slot] or 0)+1
            if infoHook then return infoHook(slot,infoCalls[slot]) end
            -- No texture, name, quality or other restricted field may be examined.
            return SECRET,SECRET,quantities[slot],ids[slot],SECRET,SECRET
        end
        function GetLootSlotLink(slot)
            linkCalls[slot]=(linkCalls[slot] or 0)+1
            if linkHook then return linkHook(slot,linkCalls[slot]) end
            return '|Hitem:4549:0|h[PRIVATE]|h'
        end
        LootSlot=forbidden; LootSlotItem=forbidden; ConfirmLootSlot=forbidden
        CloseLoot=forbidden; ClearLootSlot=forbidden; RollOnLoot=forbidden
        ConfirmLootRoll=forbidden; GetLootSourceInfo=forbidden; hooksecurefunc=forbidden
        CurrencyContainerUtil={GetCurrencyContainerInfo=forbidden}
        C_CurrencyInfo={GetCurrencyInfo=forbidden,GetCurrencyContainerInfo=forbidden}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    // The unrelated spellbook collector dereferences Enum before checking its
    // own API. Keep its access out of this loot-only secret-value fixture while
    // retaining the real manual capture, serializer, decoder and equipment row.
    lua.load("AO.collectors.spellbook=nil").exec()?;
    lua.globals().set("fixture_setup", setup)?;
    lua.load(setup).exec()?;
    Ok(lua)
}

fn capture(lua: &Lua) -> Result<Snapshot, Box<dyn std::error::Error>> {
    lua.load("SlashCmdList.AZEROTHOPERATOR('')").exec()?;
    let value: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{value}\"").as_bytes())?;
    let setup: String = lua.globals().get("fixture_setup")?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0, "{setup}");
    assert_eq!(lua.globals().get::<u32>("secretInspections")?, 0, "{setup}");
    assert_eq!(snapshot.sections["equipment"]["16"]["item_id"], 123);
    Ok(snapshot)
}

#[test]
fn typed_slots_preserve_raw_currency_and_money_without_amounts()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture("")?;
    let snapshot = capture(&lua)?;
    assert_eq!(
        snapshot.sections["loot_window"],
        json!({
            "coverage":"visible_loot_window_slots", "slot_count":3,
            "entries":{
                "1":{"status":"reported_currency","reported_currency_id":81,"reported_quantity":3},
                "2":{"status":"money"},
                "3":{"status":"item","item_id":4549,"quantity":2}
            }
        })
    );
    lua.load("assert(table.concat(trace,',')=='1,2,3,1,2,3'); assert(counts==4 and visibility==4); assert(infoCalls[1]==2 and infoCalls[2]==nil and infoCalls[3]==2); assert(linkCalls[1]==nil and linkCalls[2]==nil and linkCalls[3]==2)").exec()?;
    for (id, quantity) in [(1_u32, 0_u32), (u32::MAX, u32::MAX)] {
        let lua = fixture(&format!("ids[1]={id}; quantities[1]={quantity}"))?;
        assert_eq!(
            capture(&lua)?.sections["loot_window"]["entries"]["1"],
            json!({
                "status":"reported_currency","reported_currency_id":id,"reported_quantity":quantity
            })
        );
    }
    Ok(())
}

#[test]
fn uncertain_rows_never_become_inferred_items_or_currency() -> Result<(), Box<dyn std::error::Error>>
{
    let mut cases = vec![
        "kinds[1]=SECRET".to_owned(),
        "kinds[1]=nil".to_owned(),
        "kinds[1]='23'".to_owned(),
        "kinds[1]={}".to_owned(),
        "kinds[1]=999".to_owned(),
        "typeHook=function(s,n) if s==1 then error('PRIVATE') end return kinds[s] end".to_owned(),
        "typeHook=function(s,n) if s==1 and n==2 then return 17 end return kinds[s] end".to_owned(),
        "typeHook=function(s,n) if s==1 and n==2 then return 41 end return kinds[s] end".to_owned(),
        "typeHook=function(s,n) if s==1 and n==2 then return SECRET end return kinds[s] end".to_owned(),
        "infoHook=function(s,n) if s==1 and n==2 then error('PRIVATE') end return SECRET,SECRET,quantities[s],ids[s] end".to_owned(),
        "infoHook=function(s,n) return SECRET,SECRET,quantities[s],s==1 and (n==1 and 81 or 82) or ids[s] end".to_owned(),
        "infoHook=function(s,n) return SECRET,SECRET,s==1 and n or quantities[s],ids[s] end".to_owned(),
    ];
    for invalid in [
        "nil",
        "SECRET",
        "'1'",
        "{}",
        "-1",
        "0.5",
        "0/0",
        "math.huge",
        "4294967296",
    ] {
        cases.push(format!("ids[1]={invalid}"));
        cases.push(format!("quantities[1]={invalid}"));
    }
    cases.push("ids[1]=0".to_owned());
    for setup in cases {
        let snapshot = capture(&fixture(&setup)?)?;
        assert_eq!(
            snapshot.sections["loot_window"]["entries"]["1"],
            json!({"status":"unreadable_or_non_item"}),
            "{setup}"
        );
        assert_eq!(
            snapshot.sections["loot_window"]["entries"]["2"],
            json!({"status":"money"}),
            "{setup}"
        );
        assert_eq!(
            snapshot.sections["loot_window"]["entries"]["3"]["status"], "item",
            "{setup}"
        );
    }
    // A stable Item classification still needs an item hyperlink, never a currency link.
    let lua = fixture("kinds[1]=17; linkHook=function() return '|Hcurrency:81:|h[PRIVATE]|h' end")?;
    assert_eq!(
        capture(&lua)?.sections["loot_window"]["entries"]["1"],
        json!({"status":"unreadable_or_non_item"})
    );
    Ok(())
}

#[test]
fn invalid_typed_surfaces_are_unknown_and_absent_enums_keep_legacy_items()
-> Result<(), Box<dyn std::error::Error>> {
    for setup in [
        "Enum=SECRET",
        "Enum=42",
        "Enum.LootSlotType=SECRET",
        "Enum.LootSlotType=42",
        "Enum.LootSlotType={}",
        "Enum.LootSlotType.Item=nil",
        "Enum.LootSlotType.Currency=nil",
        "Enum.LootSlotType.Money=nil",
        "Enum.LootSlotType.Currency=SECRET",
        "Enum.LootSlotType.Item=SECRET",
        "Enum.LootSlotType.Money=SECRET",
        "Enum.LootSlotType.Currency=17",
        "Enum.LootSlotType.Currency='23'",
        "GetLootSlotType=nil",
        "GetLootSlotType=SECRET",
        "GetLootSlotType=42",
    ] {
        let snapshot = capture(&fixture(setup)?)?;
        for slot in ["1", "2", "3"] {
            assert_eq!(
                snapshot.sections["loot_window"]["entries"][slot],
                json!({"status":"unreadable_or_non_item"}),
                "{setup}: {slot}"
            );
        }
    }
    for setup in ["Enum=nil", "Enum={}"] {
        let lua = fixture(&format!("{setup}; GetLootSlotType=forbidden"))?;
        assert_eq!(
            capture(&lua)?.sections["loot_window"]["entries"]["1"],
            json!({"status":"item","item_id":4549,"quantity":3})
        );
        lua.load("assert(next(typeCalls)==nil)").exec()?;
    }
    Ok(())
}

#[test]
fn changing_context_or_api_identity_rejects_coverage() -> Result<(), Box<dyn std::error::Error>> {
    for mutation in [
        "Enum={LootSlotType=Enum.LootSlotType}",
        "Enum.LootSlotType={Item=17,Currency=23,Money=41}",
        "Enum.LootSlotType.Currency=24",
        "Enum.LootSlotType.Item=18",
        "Enum.LootSlotType.Money=42",
        "Enum.LootSlotType.Currency=SECRET",
        "Enum=nil",
        "Enum=SECRET",
        "GetLootSlotType=function(s) return kinds[s] end",
        "GetLootSlotType=SECRET",
        "GetLootSlotInfo=function(s) return SECRET,SECRET,quantities[s],ids[s] end",
        "GetLootSlotLink=function() return '|Hitem:4549:0|h[PRIVATE]|h' end",
        "GetNumLootItems=function() return 3 end",
        "LootFrame.IsShown=function() return true end",
        "LootFrame={IsShown=function() return true end}",
        "LootFrame=SECRET",
        "issecretvalue=function(v) return rawequal(v,SECRET) end",
        "AO.family='retail'",
    ] {
        let lua = fixture(&format!(
            "typeHook=function(s,n) if s==1 then {mutation} end return kinds[s] end"
        ))?;
        let snapshot = capture(&lua)?;
        assert!(!snapshot.sections.contains_key("loot_window"), "{mutation}");
        assert_eq!(
            snapshot.unavailable["loot_window"], "api_unavailable_or_failed",
            "{mutation}"
        );
    }
    for setup in [
        "visibilityHook=function() return visibility<4 end",
        "visibilityHook=function() return SECRET end",
        "countHook=function() return counts==4 and 2 or 3 end",
        "size=65",
        "LootFrame=SECRET",
        "LootFrame.IsShown=SECRET",
        "GetNumLootItems=SECRET",
        "GetLootSlotInfo=SECRET",
        "GetLootSlotLink=SECRET",
        "Enum=nil; infoHook=function(s) Enum={LootSlotType={Item=17,Currency=23,Money=41}}; return SECRET,SECRET,3,81 end",
    ] {
        assert!(
            !capture(&fixture(setup)?)?
                .sections
                .contains_key("loot_window"),
            "{setup}"
        );
    }
    // Equality metamethods cannot substitute another enum table for the captured one.
    let lua = fixture(
        "meta={__eq=function() return true end}; setmetatable(Enum,meta); typeHook=function(s) Enum=setmetatable({LootSlotType=Enum.LootSlotType},meta); return kinds[s] end",
    )?;
    assert!(!capture(&lua)?.sections.contains_key("loot_window"));
    Ok(())
}
