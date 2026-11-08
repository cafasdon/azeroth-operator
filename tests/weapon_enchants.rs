use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn weapon_enchant_lists_preserve_unknowns_and_countdown_observations()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 720 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={WeaponSlot={MainHand=0,OffHand=1,Ranged=2}}
        C_Item={GetWeaponEnchantInfo=function(slot)
            assert(slot>=0 and slot<=2);reads[slot]=(reads[slot] or 0)+1
            if slot==1 then return {{hasEnchant=false,enchantType=0,enchantID='PRIVATE',charges='PRIVATE',timeLeft='PRIVATE'}} end
            if slot==2 then return {} end
            if mode=='error' then error('PRIVATE') end
            if mode=='secret_table' then return 'PRIVATE' end
            if mode=='nil_table' then return nil end
            if mode=='oversize' then local t={};for i=1,33 do t[i]={hasEnchant=false,enchantType=0} end;return t end
            if mode=='sparse' then return {[1]={hasEnchant=false,enchantType=0},[3]={hasEnchant=false,enchantType=0}} end
            local row={hasEnchant=true,enchantType=2,enchantID=42,charges=0,timeLeft=reads[slot]>1 and 9990.2 or 10000}
            if mode=='secret_flag' then row.hasEnchant='PRIVATE' end
            if mode=='secret_kind' then row.enchantType='PRIVATE' end
            if mode=='invalid_kind' then row.enchantType=4 end
            if mode=='status_changed' and reads[slot]>1 then row.hasEnchant=false end
            if mode=='id_changed' and reads[slot]>1 then row.enchantID=43 end
            if mode=='kind_changed' and reads[slot]>1 then row.enchantType=3 end
            if mode=='secret_details' then row.enchantID='PRIVATE';row.charges='PRIVATE';row.timeLeft='PRIVATE' end
            if mode=='invalid_details' then row.enchantID=0;row.charges=-1;row.timeLeft=math.huge end
            if mode=='charges_changed' and reads[slot]>1 then row.charges=1 end
            if mode=='timer_increased' and reads[slot]>1 then row.timeLeft=10001 end
            return {row,{hasEnchant=true,enchantType=3,enchantID=44,charges=2,timeLeft=0}}
        end,UseItemByName=function() error('no use') end}
        C_Spell={CancelItemTempEnchantment=function() error('no cancel') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads={};AO.Capture()").exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    let saved = capture("normal")?;
    let hands = &saved.sections["weapon_enchants"]["hands"];
    assert_eq!(
        hands["main_hand"]["entries"]["1"]["remaining_milliseconds"],
        9991
    );
    assert_eq!(hands["main_hand"]["entries"]["1"]["charges"], 0);
    assert_eq!(hands["main_hand"]["entries"]["2"]["enchant_type"], "imbue");
    assert_eq!(
        hands["off_hand"]["entries"]["1"],
        serde_json::json!({"has_enchant":false,"enchant_type":"none"})
    );
    assert_eq!(hands["ranged"]["entries"], serde_json::json!({}));
    for mode in [
        "error",
        "secret_table",
        "nil_table",
        "oversize",
        "sparse",
        "secret_flag",
        "secret_kind",
        "invalid_kind",
        "status_changed",
        "id_changed",
        "kind_changed",
    ] {
        let saved = capture(mode)?;
        assert!(
            saved.sections["weapon_enchants"]["hands"]
                .get("main_hand")
                .is_none(),
            "{mode}"
        );
        assert!(
            saved.sections["weapon_enchants"]["hands"]
                .get("off_hand")
                .is_some()
        );
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 720);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in ["secret_details", "invalid_details"] {
        let saved = capture(mode)?;
        let row = &saved.sections["weapon_enchants"]["hands"]["main_hand"]["entries"]["1"];
        for field in ["enchant_id", "charges", "remaining_milliseconds"] {
            assert!(row.get(field).is_none(), "{mode}/{field}");
        }
        assert_eq!(row["has_enchant"], true);
    }
    assert!(capture("charges_changed")?.sections["weapon_enchants"]["hands"]["main_hand"]["entries"]["1"].get("charges").is_none());
    assert!(capture("timer_increased")?.sections["weapon_enchants"]["hands"]["main_hand"]["entries"]["1"].get("remaining_milliseconds").is_none());
    lua.load("C_Item.GetWeaponEnchantInfo=nil").exec()?;
    assert!(
        capture("normal")?
            .unavailable
            .contains_key("weapon_enchants")
    );
    Ok(())
}
