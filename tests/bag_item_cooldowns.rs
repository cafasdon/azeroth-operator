use azeroth_operator::import::decode;
use mlua::Lua;
use serde_json::json;

fn fixture(mode: &str) -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}
        infoReads={};cooldownReads={};clockReads=0;actions=0;NUM_BAG_SLOTS=1
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function forbidden() actions=actions+1;error('forbidden gameplay action') end
        UseContainerItem=forbidden;PickupContainerItem=forbidden;UseAction=forbidden
        C_Container={UseContainerItem=forbidden,PickupContainerItem=forbidden}
        function issecretvalue(value) return value=='PRIVATE' end
        function GetTime()
            clockReads=clockReads+1
            if mode=='clock_error' then error('PRIVATE') end
            if mode=='clock_secret' then return 'PRIVATE' end
            if mode=='clock_nan' then return 0/0 end
            if mode=='clock_infinite' then return math.huge end
            if mode=='clock_negative' then return -1 end
            if mode=='clock_overflow' then return 4294967296 end
            if mode=='clock_wrong_type' then return '100.2' end
            if mode=='clock_before_start' then return 89 end
            if mode=='remaining_changed' and lastCooldownSlot==1 and cooldownReads[1]==2 then return 101.2 end
            return 100.2
        end
        function C_Container.GetContainerNumSlots(bag)
            assert(bag==0 or bag==1)
            return bag==0 and mode~='empty' and 3 or 0
        end
        function C_Container.GetContainerItemInfo(bag,slot)
            assert(bag==0 and slot>=1 and slot<=3,'unrelated slot queried')
            infoReads[slot]=(infoReads[slot] or 0)+1
            local n=infoReads[slot]
            local item={itemID=100+slot,hyperlink='|Hitem:'..(100+slot)..':0|h[Item]|h',stackCount=2}
            if slot==1 and n>=2 then
                if mode=='info_error' then error('PRIVATE') end
                if mode=='info_secret' then return 'PRIVATE' end
                if mode=='info_nil' then return nil end
                if mode=='id_changed' then item.itemID=999 end
                if mode=='link_changed' then item.hyperlink='|Hitem:101:1|h[Item]|h' end
                if mode=='count_changed' then item.stackCount=3 end
                if mode=='id_secret' then item.itemID='PRIVATE' end
                if mode=='link_secret' then item.hyperlink='PRIVATE' end
                if mode=='count_secret' then item.stackCount='PRIVATE' end
                if mode=='post_identity_changed' and n>=3 then item.stackCount=3 end
            end
            return item
        end
        function C_Container.GetContainerItemCooldown(bag,slot)
            assert(bag==0 and slot>=1 and slot<=3,'unrelated cooldown queried')
            lastCooldownSlot=slot
            cooldownReads[slot]=(cooldownReads[slot] or 0)+1
            local n=cooldownReads[slot]
            if slot==2 then return 90,20.5,0 end
            if slot==3 then return 0,0,1 end
            if mode=='nil' then return nil end
            if mode=='error' then error('PRIVATE') end
            if mode=='secret_start' then return 'PRIVATE',20.5,1 end
            if mode=='secret_duration' then return 90,'PRIVATE',1 end
            if mode=='secret_enable' then return 90,20.5,'PRIVATE' end
            if mode=='wrong_start' then return '90',20.5,1 end
            if mode=='wrong_duration' then return 90,false,1 end
            if mode=='wrong_enable' then return 90,20.5,true end
            if mode=='negative_start' then return -1,20.5,1 end
            if mode=='negative_duration' then return 90,-1,1 end
            if mode=='start_overflow' then return 4294967296,20.5,1 end
            if mode=='duration_overflow' then return 90,4294967296,1 end
            if mode=='nan_start' then return 0/0,20.5,1 end
            if mode=='infinite_duration' then return 90,math.huge,1 end
            if mode=='enable_two' then return 90,20.5,2 end
            if mode=='enable_fraction' then return 90,20.5,0.5 end
            if mode=='changed_start' and n==2 then return 90.1,20.5,1 end
            if mode=='changed_duration' and n==2 then return 90,20.6,1 end
            if mode=='changed_enable' and n==2 then return 90,20.5,0 end
            if mode=='api_changed' then
                C_Container.GetContainerItemCooldown=function() return 90,20.5,1 end
            end
            if mode=='namespace_changed' then
                local replacement={};for k,v in pairs(C_Container) do replacement[k]=v end
                C_Container=replacement
            end
            if mode=='clock_pointer_changed' then GetTime=function() return 100.2 end end
            return 90,20.5,1
        end
        if mode=='lookup_error' or mode=='lookup_secret' or mode=='lookup_wrong_type' then
            C_Container.GetContainerItemCooldown=nil
            setmetatable(C_Container,{__index=function(_,key)
                if key=='GetContainerItemCooldown' then
                    if mode=='lookup_error' then error('PRIVATE') end
                    if mode=='lookup_secret' then return 'PRIVATE' end
                    return false
                end
            end})
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    // Keep the real carried-bag collector and Core dispatch, isolating unrelated collectors.
    lua.load("AO.collectors={bags=AO.collectors.bags,bag_item_cooldowns=AO.collectors.bag_item_cooldowns}")
        .exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua) -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
    lua.load("AO.Capture();assert(actions==0)").exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    Ok(decode(
        format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
    )?)
}

#[test]
fn actual_core_export_preserves_captured_locations_and_stable_public_timers()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture("normal")?;
    let saved = capture(&lua)?;
    assert_eq!(
        saved.sections["bag_item_cooldowns"],
        json!({
            "coverage":"captured_carried_items_only","captured_item_locations":3,
            "entries":{"0":{
                "1":{"item_id":101,"status":"reported","enabled":true,"remaining_seconds":11},
                "2":{"item_id":102,"status":"reported","enabled":false,"remaining_seconds":11},
                "3":{"item_id":103,"status":"reported","enabled":true,"remaining_seconds":0}
            },"1":{}}
        })
    );
    lua.load("for slot=1,3 do assert(cooldownReads[slot]==2);assert(infoReads[slot]==5) end;assert(clockReads==6)").exec()?;
    assert_eq!(
        capture(&fixture("empty")?)?.sections["bag_item_cooldowns"],
        json!({
            "coverage":"captured_carried_items_only","captured_item_locations":0,"entries":{"0":{},"1":{}}
        })
    );
    Ok(())
}

#[test]
fn uncertain_or_changed_reads_keep_an_unavailable_item_row()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "nil",
        "error",
        "secret_start",
        "secret_duration",
        "secret_enable",
        "wrong_start",
        "wrong_duration",
        "wrong_enable",
        "negative_start",
        "negative_duration",
        "start_overflow",
        "duration_overflow",
        "nan_start",
        "infinite_duration",
        "enable_two",
        "enable_fraction",
        "changed_start",
        "changed_duration",
        "changed_enable",
        "remaining_changed",
        "clock_error",
        "clock_secret",
        "clock_nan",
        "clock_infinite",
        "clock_negative",
        "clock_overflow",
        "clock_wrong_type",
        "clock_before_start",
        "info_error",
        "info_secret",
        "info_nil",
        "id_changed",
        "link_changed",
        "count_changed",
        "id_secret",
        "link_secret",
        "count_secret",
        "post_identity_changed",
        "api_changed",
        "namespace_changed",
        "clock_pointer_changed",
        "lookup_error",
        "lookup_secret",
        "lookup_wrong_type",
    ] {
        let saved = capture(&fixture(mode)?)?;
        assert_eq!(
            saved.sections["bag_item_cooldowns"]["entries"]["0"]["1"],
            json!({"item_id":101,"status":"unavailable"}),
            "{mode}"
        );
        assert_eq!(
            saved.sections["bag_item_cooldowns"]["captured_item_locations"], 3,
            "{mode}"
        );
        assert!(
            !serde_json::to_string(&saved)?.contains("PRIVATE"),
            "{mode}"
        );
    }
    Ok(())
}

#[test]
fn direct_collector_rejects_bad_scope_and_bounds_before_api_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture("normal")?;
    lua.load(
        r#"
        local collect=AO.collectors.bag_item_cooldowns
        local function rejected(value) assert(not pcall(collect,value)) end
        rejected(nil);rejected({['00']={}});rejected({['-1']={}});rejected({['65']={}})
        rejected({['0']={['0']={item_id=1}}});rejected({['0']={['01']={item_id=1}}})
        rejected({['0']={['1025']={item_id=1}}});rejected({['0']={['1']={item_id=0}}})
        local oversized={}
        for bag=0,3 do
            oversized[tostring(bag)]={}
            for slot=1,1024 do oversized[tostring(bag)][tostring(slot)]={item_id=1} end
        end
        oversized['4']={['1']={item_id=1}};rejected(oversized)
        assert(next(cooldownReads)==nil and next(infoReads)==nil and clockReads==0)
        for _,item in ipairs({{item_id=101,count=2},{item_id=101,link='|Hitem:101:0|h[Item]|h'},
            {item_id=101,count=0,link='|Hitem:101:0|h[Item]|h'},
            {item_id=101,count='PRIVATE',link='|Hitem:101:0|h[Item]|h'}}) do
            local result=collect({['0']={['1']=item}})
            assert(result.entries['0']['1'].status=='unavailable')
        end
        assert(next(cooldownReads)==nil and next(infoReads)==nil and clockReads==0)
        AO.family='retail';rejected({});AO.family='forever'
        InCombatLockdown=function() return true end;rejected({})
        assert(not pcall(AO.Capture));InCombatLockdown=function() return false end
        C_Container.GetContainerItemCooldown=nil;rejected({})
        assert(actions==0)
    "#,
    )
    .exec()?;
    let missing = capture(&lua)?;
    assert!(missing.unavailable.contains_key("bag_item_cooldowns"));
    lua.load("AO.family='retail'").exec()?;
    let other = capture(&lua)?;
    assert!(!other.sections.contains_key("bag_item_cooldowns"));
    assert!(!other.unavailable.contains_key("bag_item_cooldowns"));
    Ok(())
}
