use mlua::Lua;
#[test]
fn merge_decoder_requires_matching_descriptions_and_distinct_instances() -> mlua::Result<()> {
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    lua.load(include_str!("../addon/BagMergeImport.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
 local function hex(s) return (s:gsub('.',function(c) return string.format('%02x',string.byte(c)) end)) end
 local fields={'AO-BAGMERGE-1',string.rep('a',64),string.rep('b',64),'retail',hex('live'),hex('70688'),'120105',hex('Test'),hex('Test'),'0','1','123','2',hex('one'),hex('item:123'),'1','2','123','3',hex('two'),hex('item:123')}
 local function decode() return AO.DecodeBagMerge(table.concat(fields,':')) end
 local op=decode();assert(op.kind=='merge_bag_items' and op.source.count==2 and op.target.count==3)
 fields[20]=hex('one');assert(not pcall(decode));fields[20]=hex('two')
 fields[18]='456';assert(not pcall(decode));fields[18]='123'
 fields[21]=hex('item:123:other');assert(not pcall(decode));fields[21]=hex('item:123')
 fields[19]='03';assert(not pcall(decode));fields[19]='3'
 fields[1]='AO-BAGSWAP-1';assert(not pcall(decode))
 "#).exec()
}

#[test]
fn compiler_to_review_apply_and_receipt_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use azeroth_operator::{
        Snapshot,
        bag_operation::{BagRequest, compile},
    };
    use serde_json::json;

    let mut snapshot: Snapshot = serde_json::from_value(json!({
        "schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Test","realm":"Test"},
        "sections":{"bags":{"0":{"1":{"item_id":123,"count":2,"guid":"one","link":"item:123"}},
                            "1":{"2":{"item_id":123,"count":3,"guid":"two","link":"item:123"}}}}
    }))?;
    let request: BagRequest = serde_json::from_value(json!({
        "operation":"merge","bag":0,"slot":1,"target_bag":1,"target_slot":2
    }))?;
    let prepare = |snapshot: &Snapshot| compile(snapshot, &"a".repeat(64), &request);
    let draft = prepare(&snapshot)?;
    assert_eq!(draft["kind"], "merge_bag_items");
    assert_eq!(draft["addon_execution_enabled"], false);
    snapshot.sections.get_mut("bags").unwrap()["1"]["2"]["count"] = json!(2147483647_u32);
    assert!(prepare(&snapshot).is_err());

    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.globals().set("AO", ao.clone())?;
    lua.globals()
        .set("code", draft["import_code"].as_str().unwrap())?;
    lua.globals().set("SlashCmdList", lua.create_table()?)?;
    for source in [
        include_str!("../addon/BagMergeImport.lua"),
        include_str!("../addon/BagCompatibility.lua"),
        include_str!("../addon/BagMerge.lua"),
        include_str!("../addon/MacroImport.lua"),
        include_str!("../addon/Core.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(r#"
        AO.family='retail';AO.variant='live';AO.interface=120105
        NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        function GetBuildInfo() return '', '70688','',120105 end
        function UnitName() return 'Test' end;function GetRealmName() return 'Test' end
        function InCombatLockdown() return false end
        function UnitCastingInfo() end;function UnitChannelInfo() end
        function GetCursorInfo() if cursor then return 'item',cursor.itemID,cursor.hyperlink end end
        function GetTime() return now end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return bag..':'..slot end}
        C_Item={GetItemGUID=function(key) return slots[key] and slots[key].guid end,
            GetItemMaxStackSize=function() return 20 end}
        C_Container={GetContainerNumSlots=function() return 10 end,
            GetContainerItemInfo=function(bag,slot) return slots[bag..':'..slot] end}
        now=0;writes=0
        function reset()
            slots={['0:1']={itemID=123,stackCount=2,guid='one',hyperlink='item:123'},
                ['1:2']={itemID=123,stackCount=3,guid='two',hyperlink='item:123'}}
            cursor=nil;delayed=nil;explode=nil
        end
        function C_Container.PickupContainerItem(bag,slot)
            assert(AzerothOperatorOperations:find(':outcome_unknown:merge_bag_items;',1,true))
            writes=writes+1
            local key=bag..':'..slot
            if not cursor then
                cursor=slots[key];slots[key]=nil
                if explode then error('pickup effect occurred') end
            else
                if not delayed then slots[key].stackCount=slots[key].stackCount+cursor.stackCount end
                cursor=nil
            end
        end
        function fresh(letter)
            return (code:gsub('^AO%-BAGMERGE%-1:[0-9a-f]+:', 'AO-BAGMERGE-1:'..string.rep(letter,64)..':'))
        end
    "#).exec()?;
    lua.load(include_str!("bag_metadata_fixture.lua")).exec()?;
    lua.load(r#"
        reset()
        assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        local op=AO.ReviewMacroImport(code)
        assert(op.source.guid=='one' and op.target.guid=='two' and op.target.bag==1 and writes==0)
        assert(not pcall(AO.ApplyReviewedMacro,code..':') and writes==0)
        AO.ReviewMacroImport(code)
        local receipt,ok=AO.ApplyReviewedMacro(code)
        assert(ok and receipt.status=='applied' and writes==2 and not slots['0:1'])
        assert(slots['1:2'].guid=='two' and slots['1:2'].stackCount==5 and not cursor)

        for _,case in ipairs({{'b',10,'applied'},{'c',61,'outcome_unknown'}}) do
            reset();now=0;delayed=true
            local attempt=fresh(case[1]);AO.ReviewMacroImport(attempt)
            receipt,ok=AO.ApplyReviewedMacro(attempt)
            assert(ok and receipt.status=='outcome_unknown')
            local before=writes;now=case[2];slots['1:2'].stackCount=5
            assert(AO.ReadMacroReceipts()[string.rep(case[1],64)].status==case[3] and writes==before)
        end

        reset();explode=true
        local failed=fresh('d');AO.ReviewMacroImport(failed);receipt,ok=AO.ApplyReviewedMacro(failed)
        assert(not ok and receipt.status=='outcome_unknown' and writes==7 and cursor.guid=='one')
        AO.collectors={operation_receipts=AO.ReadMacroReceipts}
        AO.Capture()
    "#).exec()?;
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
        receipts[draft["operation_id"].as_str().unwrap()],
        json!({
            "kind":"merge_bag_items","status":"applied"
        })
    );
    assert_eq!(receipts["b".repeat(64)]["status"], "applied");
    for letter in ["c", "d"] {
        assert_eq!(receipts[letter.repeat(64)]["status"], "outcome_unknown");
    }
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
        for _,attempt in ipairs({code,fresh('c'),fresh('d')}) do
            AO.ReviewMacroImport(attempt)
            assert(not pcall(AO.ApplyReviewedMacro,attempt) and writes==7)
        end
    "#,
    )
    .exec()?;
    Ok(())
}
