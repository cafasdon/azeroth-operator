use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn native_set_catalogue_preserves_unknowns_and_refuses_partial_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='retail',variant='live'}; SlashCmdList={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '12.1.5','70688','',120105 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Fixture' end
        function GetServerTime() return 0 end
        function date() return '2025-01-01T00:00:00Z' end
        ids={0,7}; count=2; itemIDs={[16]=123,[17]=0}; locations={[16]=1048592,[17]=-1,[18]=4294967295}
        ignored={[1]=true,[16]=false}
        C_EquipmentSet={
            CanUseEquipmentSets=function() return true end,
            GetNumEquipmentSets=function() return count end,
            GetEquipmentSetIDs=function() return ids end,
            GetEquipmentSetInfo=function(id) return 'Set '..id,134400,id,false,1,0,0,1,1 end,
            GetIgnoredSlots=function() return ignored end,
            GetItemIDs=function() return itemIDs end,
            GetItemLocations=function() return locations end,
            GetEquipmentSetAssignedSpec=function(id) if id==7 then return 2 end end,
        }
    "#,
    )
    .exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load("AO.Capture()").exec()?;
    let hex: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{hex}\"").as_bytes())?;
    let sets = &snapshot.sections["equipment_sets"];
    assert_eq!(sets["entries"].as_object().unwrap().len(), 2);
    assert_eq!(sets["entries"]["0"]["num_lost"], 1);
    assert!(sets["entries"]["0"].get("assigned_spec").is_none());
    assert_eq!(sets["entries"]["7"]["assigned_spec"], 2);
    let slots = &sets["entries"]["0"]["slots"];
    assert_eq!(slots["1"]["ignored"], true);
    assert_eq!(slots["16"]["ignored"], false);
    assert_eq!(slots["17"]["raw_item_id"], 0);
    assert_eq!(slots["17"]["raw_location"], -1);
    assert_eq!(slots["18"]["raw_location"], 4294967295_u64);
    assert!(slots["2"].as_object().unwrap().is_empty());

    for (field, value) in [
        ("coverage", serde_json::json!("complete_exact_items")),
        ("can_use", serde_json::json!(1)),
    ] {
        let mut malformed = snapshot.clone();
        malformed.sections.get_mut("equipment_sets").unwrap()[field] = value;
        assert!(malformed.validate().is_err());
    }
    for (field, value) in [
        ("ignored", serde_json::json!(null)),
        ("raw_location", serde_json::json!(9007199254740992_i64)),
        ("raw_item_id", serde_json::json!(1.5)),
        ("empty", serde_json::json!(true)),
    ] {
        let mut malformed = snapshot.clone();
        malformed.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["slots"]["2"]
            [field] = value;
        assert!(malformed.validate().is_err());
    }
    let mut incomplete = snapshot.clone();
    incomplete.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["slots"]
        .as_object_mut()
        .unwrap()
        .remove("19");
    assert!(incomplete.validate().is_err());
    lua.load(r#"
        NUM_TOTAL_EQUIPPED_BAG_SLOTS=5
        C_Container={GetContainerNumSlots=function() return 20 end}
        ItemLocation={CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end,
            CreateFromEquipmentSlot=function(_,slot) return {slot=slot} end}
        C_Item={GetItemID=function() return 123 end,
            GetItemGUID=function(loc) return loc.bag and 'Item-Bag-Duplicate' or 'Item-Worn-Duplicate' end,
            IsLocked=function(loc) return loc.bag~=nil end}
        EquipmentManager_GetLocationData=function(raw)
            if raw==1048592 then return {isPlayer=true,isBank=false,isBags=false,slot=16} end
            if raw==999 then return {isPlayer=true,isBank=false,isBags=true,bag=0,slot=3} end
            return {}
        end
        itemIDs[15]=123; locations[15]=999; ignored[15]=false
        AO.Capture()
    "#).exec()?;
    let identity_hex: String = lua.globals().get("AzerothOperatorExport")?;
    let identity_snapshot =
        decode(format!("AzerothOperatorExport = \"{identity_hex}\"").as_bytes())?;
    let identity_slots = &identity_snapshot.sections["equipment_sets"]["entries"]["0"]["slots"];
    assert_eq!(
        identity_slots["16"]["observed_item"]["guid"],
        "Item-Worn-Duplicate"
    );
    assert_eq!(
        identity_slots["15"]["observed_item"]["guid"],
        "Item-Bag-Duplicate"
    );
    assert_eq!(identity_slots["15"]["observed_item"]["bag"], 0);
    assert_eq!(identity_slots["15"]["observed_item"]["locked"], true);
    for (field, value) in [
        ("kind", serde_json::json!("bank")),
        ("bag", serde_json::json!(0)),
        ("guid", serde_json::json!("bad\nidentity")),
        ("item_id", serde_json::json!(456)),
        ("locked", serde_json::json!(null)),
    ] {
        let mut malformed = identity_snapshot.clone();
        malformed.sections.get_mut("equipment_sets").unwrap()["entries"]["0"]["slots"]["16"]["observed_item"]
            [field] = value;
        assert!(malformed.validate().is_err());
    }
    lua.load(r#"
        local read=AO.collectors.equipment_sets
        local function noIdentity() assert(read().entries['0'].slots['16'].observed_item==nil) end
        ignored[16]=true; noIdentity(); ignored[16]=false
        local getID=C_Item.GetItemID
        C_Item.GetItemID=function() return 456 end; noIdentity(); C_Item.GetItemID=getID
        local getGUID=C_Item.GetItemGUID
        C_Item.GetItemGUID=function() return nil end; noIdentity()
        C_Item.GetItemGUID=function() error('unavailable') end; noIdentity()
        local serial=0
        C_Item.GetItemGUID=function() serial=serial+1; return 'Changing-'..serial end; noIdentity()
        C_Item.GetItemGUID=getGUID
        issecretvalue=function(v) return v=='Item-Worn-Duplicate' end; noIdentity(); issecretvalue=nil
        local decoder=EquipmentManager_GetLocationData
        EquipmentManager_GetLocationData=nil; noIdentity()
        EquipmentManager_GetLocationData=function() return {isPlayer=true,isBank=true,isBags=false,slot=16} end; noIdentity()
        EquipmentManager_GetLocationData=function() return {isPlayer=true,isBank=false,isBags=true,bag=6,slot=1} end; noIdentity()
        EquipmentManager_GetLocationData=function() return {isPlayer=true,isBank=false,isBags=true,bag=0,slot=21} end; noIdentity()
        EquipmentManager_GetLocationData=decoder
        assert(read().entries['0'].slots['16'].observed_item.guid=='Item-Worn-Duplicate')
    "#).exec()?;
    lua.load(
        r#"
        local read=AO.collectors.equipment_sets
        ids={0,0}; assert(not pcall(read)); ids={0,7}
        count=1; assert(not pcall(read)); count=2
        ignored=nil; assert(not pcall(read)); ignored={[1]=true}
        locations[20]=1; assert(not pcall(read)); locations[20]=nil
        locations[18]=4294967296; assert(not pcall(read)); locations[18]=4294967295
        itemIDs[16]=1.5; assert(not pcall(read)); itemIDs[16]=123
        issecretvalue=function(v) return v==123 end
        assert(not pcall(read)); issecretvalue=nil
        AO.family='classic'; assert(not pcall(read)); AO.family='retail'
        local getIDs=C_EquipmentSet.GetEquipmentSetIDs
        local calls=0
        C_EquipmentSet.GetEquipmentSetIDs=function()
            calls=calls+1; return calls==1 and {0,7} or {0,8}
        end
        assert(not pcall(read)); C_EquipmentSet.GetEquipmentSetIDs=getIDs
        local getInfo=C_EquipmentSet.GetEquipmentSetInfo
        C_EquipmentSet.GetEquipmentSetInfo=function() return nil end
        AO.Capture()
        C_EquipmentSet.GetEquipmentSetInfo=getInfo
    "#,
    )
    .exec()?;
    let failed_hex: String = lua.globals().get("AzerothOperatorExport")?;
    let failed = decode(format!("AzerothOperatorExport = \"{failed_hex}\"").as_bytes())?;
    assert!(!failed.sections.contains_key("equipment_sets"));
    assert_eq!(
        failed.unavailable["equipment_sets"],
        "api_unavailable_or_failed"
    );
    lua.load("ids={}; count=0; AO.Capture()").exec()?;
    let empty_hex: String = lua.globals().get("AzerothOperatorExport")?;
    let empty = decode(format!("AzerothOperatorExport = \"{empty_hex}\"").as_bytes())?;
    assert!(
        empty.sections["equipment_sets"]["entries"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    Ok(())
}
