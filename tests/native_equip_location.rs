use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn equip_locations_are_optional_link_bound_and_stable() -> Result<(), Box<dyn std::error::Error>> {
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
        function GetInventoryItemID(_,slot) if slot==11 then return 720 end end
        function GetInventoryItemLink() if mode~='no_link' then return 'item:720:0:0:0:0:0:0' end end
        C_Container={GetContainerNumSlots=function() return 1 end,GetContainerItemInfo=function()
            return {itemID=720,stackCount=1,hyperlink=GetInventoryItemLink()} end}
        C_Item={GetItemInfoInstant=function(link)
            assert(link=='item:720:0:0:0:0:0:0');reads=reads+1
            if mode=='error' then error('PRIVATE') end
            local id=mode=='wrong_id' and 721 or mode=='secret_id' and 'PRIVATE' or 720
            local location='INVTYPE_FINGER'
            if mode=='missing' then location=nil end
            if mode=='secret' then location='PRIVATE' end
            if mode=='empty' then location='' end
            if mode=='new_type' then location='INVTYPE_FUTURE_99' end
            if mode=='invalid' then location='finger' end
            if mode=='oversize' then location='INVTYPE_'..string.rep('A',57) end
            if mode=='changed' and reads%2==0 then location='INVTYPE_TRINKET' end
            return id,'PRIVATE','PRIVATE',location,'PRIVATE','PRIVATE','PRIVATE'
        end,RequestLoadItemDataByID=function() error('load forbidden') end,
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
    for (mode, expected) in [
        ("normal", "INVTYPE_FINGER"),
        ("empty", ""),
        ("new_type", "INVTYPE_FUTURE_99"),
    ] {
        let saved = capture(mode)?;
        assert_eq!(
            saved.sections["equipment"]["11"]["equip_location"],
            expected
        );
        assert_eq!(saved.sections["bags"]["0"]["1"]["equip_location"], expected);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in [
        "no_link",
        "wrong_id",
        "secret_id",
        "missing",
        "secret",
        "invalid",
        "oversize",
        "changed",
        "error",
    ] {
        let saved = capture(mode)?;
        assert_eq!(saved.sections["equipment"]["11"]["item_id"], 720);
        assert!(
            saved.sections["equipment"]["11"]
                .get("equip_location")
                .is_none(),
            "{mode}"
        );
        assert!(
            saved.sections["bags"]["0"]["1"]
                .get("equip_location")
                .is_none(),
            "{mode}"
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("C_Item.GetItemInfoInstant=nil").exec()?;
    assert!(
        capture("normal")?.sections["equipment"]["11"]
            .get("equip_location")
            .is_none()
    );
    Ok(())
}
