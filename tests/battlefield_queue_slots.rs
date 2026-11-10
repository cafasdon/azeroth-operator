use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str) -> Result<(Snapshot,Lua),Box<dyn std::error::Error>> {
    let lua=Lua::new();lua.globals().set("mode",mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};reads=0;counts=0;actions=0;SECRET={}
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function forbidden() actions=actions+1;error('forbidden queue action') end
        function GetMaxBattlefieldID()
            counts=counts+1
            if mode=='zero' then return 0 end
            if mode=='maximum' then return 32 end
            if mode=='too_many' then return 33 end
            if mode=='bad_count' then return 1.5 end
            if mode=='secret_count' then return SECRET end
            if mode=='changing_count' and counts>1 then return 4 end
            return 5
        end
        function GetBattlefieldStatus(slot)
            reads=reads+1
            local statuses={'none','queued','confirm','active','locked'}
            local status=statuses[(slot-1)%5+1]
            if mode=='changing_status' and reads>5 then status='none' end
            if mode=='unknown' then status='PRIVATE' end
            if mode=='missing' then return nil end
            if mode=='restricted' then return SECRET end
            if mode=='error' then error('PRIVATE') end
            if mode=='replace_info' then GetBattlefieldStatus=function() return status end end
            if mode=='replace_count' then GetMaxBattlefieldID=function() return 5 end end
            if mode=='replace_guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
            if mode=='replace_family' then AO.family='retail' end
            return status,SECRET,SECRET,SECRET,SECRET,SECRET,'PRIVATE'
        end
        JoinBattlefield=forbidden;AcceptBattlefieldPort=forbidden;LeaveBattlefield=forbidden
        RequestBattlefieldScoreData=forbidden;GetBattlefieldScore=forbidden
        GetBattlefieldTimeWaited=forbidden;GetBattlefieldEstimatedWaitTime=forbidden
        GetWorldPVPQueueStatus=forbidden;GetLFGMode=forbidden
        hooksecurefunc=forbidden
    "#).exec()?;
    let ns:mlua::Table=lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua")).call::<()>(("AzerothOperator",ns.clone()))?;
    lua.load(include_str!("../addon/Core.lua")).call::<()>(("AzerothOperator",ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported:String=lua.globals().get("AzerothOperatorExport")?;
    let snapshot=decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?,0);
    Ok((snapshot,lua))
}

#[test]
fn status_only_two_pass_capture_and_zero_capacity() -> Result<(),Box<dyn std::error::Error>> {
    let (saved,lua)=capture("normal")?;
    assert_eq!(saved.sections["battlefield_queue_slots"],json!({
        "coverage":"reported_battlefield_queue_slots","reported_slot_count":5,"entries":{
            "1":{"reported_status":"none"},"2":{"reported_status":"queued"},
            "3":{"reported_status":"confirm"},"4":{"reported_status":"active"},"5":{"reported_status":"locked"}}}));
    assert_eq!(lua.globals().get::<u32>("reads")?,10);
    assert_eq!(capture("zero")?.0.sections["battlefield_queue_slots"],json!({
        "coverage":"reported_battlefield_queue_slots","reported_slot_count":0,"entries":{}}));
    assert_eq!(capture("maximum")?.0.sections["battlefield_queue_slots"]["entries"].as_object().unwrap().len(),32);
    Ok(())
}

#[test]
fn changed_restricted_missing_or_unsupported_queues_are_unavailable() -> Result<(),Box<dyn std::error::Error>> {
    for mode in ["too_many","bad_count","secret_count","changing_count","changing_status","unknown",
        "missing","restricted","error","replace_info","replace_count","replace_guard","replace_family"] {
        let (saved,_)=capture(mode)?;
        assert!(!saved.sections.contains_key("battlefield_queue_slots"),"{mode}");
        assert_eq!(saved.unavailable["battlefield_queue_slots"],"api_unavailable_or_failed");
    }
    Ok(())
}
