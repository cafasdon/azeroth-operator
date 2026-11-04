use azeroth_operator::{
    Snapshot,
    bag_operation::{BagRequest, compile},
};
use mlua::Lua;
use serde_json::json;
#[test]
fn swap_transport_binds_both_items_and_rejects_ambiguous_requests()
-> Result<(), Box<dyn std::error::Error>> {
    verify_swap_fixture(None)
}
pub fn verify_swap_fixture(
    root: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
 "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},
 "sections":{"bags":{"0":{"1":{"item_id":123,"count":2,"guid":"one","link":"source:|link"},"2":{"item_id":456,"count":3,"guid":"two","link":"target:|link"}}}}}),
    )?;
    let mut request: BagRequest = serde_json::from_value(
        json!({"operation":"swap","bag":0,"slot":1,"target_bag":0,"target_slot":2}),
    )?;
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    assert_eq!(prepared["kind"], "swap_bag_items");
    assert_eq!(prepared["addon_execution_enabled"], true);
    assert_eq!(prepared["requires_addon_version"], "0.19.0-dev");
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.load(fixture_source(
        root,
        "BagSwapImport.lua",
        include_str!("../addon/BagSwapImport.lua"),
    )?)
    .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeBagSwap")?;
    let code = prepared["import_code"].as_str().unwrap();
    let op: mlua::Table = decoder.call(code)?;
    assert_eq!(
        op.get::<mlua::Table>("source")?.get::<String>("link")?,
        "source:|link"
    );
    assert_eq!(
        op.get::<mlua::Table>("target")?.get::<String>("guid")?,
        "two"
    );
    for (field, bad) in [
        (9, "00"),
        (10, "0"),
        (11, "4294967296"),
        (13, ""),
        (14, "zz"),
        (17, "123"),
        (19, "6f6e65"),
    ] {
        let mut fields: Vec<_> = code.split(':').collect();
        fields[field] = bad;
        assert!(decoder.call::<mlua::Table>(fields.join(":")).is_err());
    }
    assert!(
        decoder
            .call::<mlua::Table>(format!("{code}:extra"))
            .is_err()
    );
    lua.globals().set("AO", ao.clone())?;
    lua.globals().set("code", code)?;
    for source in [
        fixture_source(root, "BagSwap.lua", include_str!("../addon/BagSwap.lua"))?,
        fixture_source(
            root,
            "MacroImport.lua",
            include_str!("../addon/MacroImport.lua"),
        )?,
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(r#"
        AO.family='retail'; AO.variant='live'; AO.interface=120105; NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        function GetBuildInfo() return '', '70688','',120105 end
        function UnitName() return 'Test' end; function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end;function UnitChannelInfo() end
        function GetCursorInfo() if cursor then return 'item',cursor.itemID,cursor.hyperlink end end
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return slot end}
        C_Item={GetItemGUID=function(slot) return slots[slot] and slots[slot].guid end}
        C_Container={GetContainerNumSlots=function() return 10 end,GetContainerItemInfo=function(_,slot) return slots[slot] end}
        now=0;writes=0;function GetTime() return now end
        function reset()
            slots={[1]={itemID=123,stackCount=2,guid='one',hyperlink='source:|link'},[2]={itemID=456,stackCount=3,guid='two',hyperlink='target:|link'}}
            cursor=nil;calls=0;delay=nil;explode=nil
        end
        function C_Container.PickupContainerItem(_,slot)
            assert(AzerothOperatorOperations:find(':outcome_unknown:swap_bag_items;',1,true))
            writes=writes+1;calls=calls+1
            if explode=='before' then error('before effect') end
            if delay and calls==3 then pending=cursor;cursor=nil;return end
            slots[slot],cursor=cursor,slots[slot]
            if explode=='after' then error('after effect') end
        end
        function fresh(letter) reset();return code:gsub('AO%-BAGSWAP%-1:[0-9a-f]+:','AO-BAGSWAP-1:'..string.rep(letter,64)..':',1) end
        reset();assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code);assert(not pcall(AO.ApplyReviewedMacro,code..':') and writes==0)
        AO.ReviewMacroImport(code);local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and writes==3 and slots[1].guid=='two' and slots[2].guid=='one')
        AO.ReviewMacroImport(code);assert(not pcall(AO.ApplyReviewedMacro,code) and writes==3)
        local delayed=fresh('b');delay=true;AO.ReviewMacroImport(delayed);receipt,ok=AO.ApplyReviewedMacro(delayed)
        assert(ok and receipt.status=='outcome_unknown' and writes==6)
        assert(AO.ReadMacroReceipts()[string.rep('b',64)].status=='outcome_unknown')
        slots[1]=pending;now=10;assert(AO.ReadMacroReceipts()[string.rep('b',64)].status=='applied' and writes==6)
        local expired=fresh('c');delay=true;AO.ReviewMacroImport(expired);AO.ApplyReviewedMacro(expired)
        slots[1]=pending;now=71;assert(AO.ReadMacroReceipts()[string.rep('c',64)].status=='outcome_unknown' and writes==9)
        for _,pair in ipairs({{'d','before'},{'e','after'}}) do
            local failed=fresh(pair[1]);explode=pair[2];AO.ReviewMacroImport(failed);receipt,ok=AO.ApplyReviewedMacro(failed)
            assert(not ok and receipt.status=='outcome_unknown')
        end
        assert(writes==11)
        SlashCmdList={};function GetServerTime() return 1735689600 end;function date() return '2025-01-01T00:00:00Z' end
        AO.collectors={operation_receipts=AO.ReadMacroReceipts}
    "#).exec()?;
    lua.load(fixture_source(
        root,
        "Core.lua",
        include_str!("../addon/Core.lua"),
    )?)
    .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.Capture()").exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let checkpoint: String = lua.globals().get("AzerothOperatorOperations")?;
    let imported = azeroth_operator::import::decode(
        format!(
            "AzerothOperatorExport = \"{export}\"\nAzerothOperatorOperations = \"{checkpoint}\""
        )
        .as_bytes(),
    )?;
    let receipts = &imported.sections["operation_receipts"];
    assert_eq!(
        receipts[prepared["operation_id"].as_str().unwrap()],
        json!({"kind":"swap_bag_items","status":"applied"})
    );
    assert_eq!(receipts["b".repeat(64)]["status"], "applied");
    for letter in ["c", "d", "e"] {
        assert_eq!(receipts[letter.repeat(64)]["status"], "outcome_unknown");
    }
    lua.load(fixture_source(
        root,
        "MacroImport.lua",
        include_str!("../addon/MacroImport.lua"),
    )?)
    .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(
        r#"
        for _,letter in ipairs({'c','d','e'}) do
            local attempted=fresh(letter);AO.ReviewMacroImport(attempted)
            assert(not pcall(AO.ApplyReviewedMacro,attempted) and writes==11)
            assert(AO.ReadMacroReceipts()[string.rep(letter,64)].status=='outcome_unknown')
        end
    "#,
    )
    .exec()?;
    request.expect_empty_target = true;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.expect_empty_target = false;
    request.amount = Some(1);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.amount = None;
    request.target_slot = 3;
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    request.target_slot = 2;
    snapshot.sections.get_mut("bags").unwrap()["0"]["2"]["count"] = json!(4);
    assert_ne!(
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot.sections.get_mut("bags").unwrap()["0"]["2"]["link"] = json!(null);
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}

fn fixture_source(
    root: Option<&std::path::Path>,
    name: &str,
    embedded: &str,
) -> std::io::Result<String> {
    match root {
        Some(path) => std::fs::read_to_string(path.join(name)),
        None => Ok(embedded.to_owned()),
    }
}
