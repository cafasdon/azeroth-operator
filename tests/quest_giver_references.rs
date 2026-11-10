use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str) -> Result<(Snapshot, Lua), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(
        r#"
        AO={family='forever',variant='live'};SlashCmdList={};actions=0;reads=0;SECRET={}
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player');return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        local statuses=0;local existsReads={}
        LibQuestieDB={RequireContract=function(v) assert(v==1);return true end,
            ModeIndicator={GetStatus=function()
                statuses=statuses+1
                return {expansion=mode=='wrong_flavour' and 'Vanilla' or 'Forever',mode='source',
                    contractVersion=mode=='metadata_change' and statuses or 1}
            end},Quest={}}
        function LibQuestieDB.Quest.Exists(id)
            reads=reads+1;existsReads[id]=(existsReads[id] or 0)+1
            if mode=='restricted' then return SECRET end
            if id==4 then return existsReads[id]%2==1 end
            return id~=3
        end
        function LibQuestieDB.Quest.Get(id,field)
            assert(field=='startedBy' or field=='finishedBy')
            if mode=='replace_api' then LibQuestieDB.Quest.Get=function() return {} end end
            if id==5 then error('PRIVATE_PROVIDER_MESSAGE') end
            if id==6 then return nil end
            if mode=='malformed' then return {[1]={[2]=7}} end
            if field=='finishedBy' then return {[1]={99},[2]={100}} end
            local ids={};for index=20,1,-1 do ids[#ids+1]=index end;ids[#ids+1]=1
            return {[1]=ids,[2]={55},[3]={66}}
        end
        function forbidden() actions=actions+1;error('Forbidden quest action') end
        AcceptQuest=forbidden;CompleteQuest=forbidden;GetQuestReward=forbidden
        C_QuestLog={SetSelectedQuest=forbidden,RequestLoadQuestByID=forbidden}
        if mode=='absent' then LibQuestieDB=nil end
    "#,
    )
    .exec()?;
    let ns: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", ns.clone()))?;
    lua.load(r#"
        AO.collectors.quests=function()
            local result={}
            if mode~='empty' then for id=1,66 do result[tostring(id)]={title='Fixture',level=1} end end
            return result
        end
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let saved = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok((saved, lua))
}

#[test]
fn giver_references_roundtrip_with_explicit_scope_and_unknowns()
-> Result<(), Box<dyn std::error::Error>> {
    let (saved, lua) = capture("normal")?;
    let section = &saved.sections["quest_giver_references"];
    assert_eq!(section["reported_quest_count"], 66);
    let rows = section["entries"].as_object().unwrap();
    assert_eq!(rows.len(), 64);
    assert!(!rows.contains_key("65"));
    assert!(!rows.contains_key("66"));
    let row = &rows["1"];
    assert_eq!(row["started_by"]["npcs"]["distinct_id_count"], 20);
    assert_eq!(row["started_by"]["npcs"]["truncated"], true);
    let ids = row["started_by"]["npcs"]["entries"].as_object().unwrap();
    assert_eq!(ids.len(), 16);
    for id in 1..=16 {
        assert_eq!(ids[&id.to_string()], true);
    }
    assert_eq!(row["started_by"]["objects"]["entries"], json!({"55":true}));
    assert_eq!(row["started_by"]["items"]["entries"], json!({"66":true}));
    assert_eq!(row["finished_by"]["npcs"]["entries"], json!({"99":true}));
    assert_eq!(rows["3"], json!({"status":"not_in_provider"}));
    assert_eq!(rows["4"], json!({"status":"unavailable"}));
    assert_eq!(rows["5"], json!({"status":"reported"}));
    assert_eq!(rows["6"], json!({"status":"reported"}));
    assert_eq!(lua.globals().get::<u32>("reads")?, 128);
    Ok(())
}

#[test]
fn provider_failures_never_become_quest_absence_or_actions()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["absent", "wrong_flavour", "metadata_change", "replace_api"] {
        let (saved, _) = capture(mode)?;
        assert!(
            !saved.sections.contains_key("quest_giver_references"),
            "{mode}"
        );
        assert_eq!(
            saved.unavailable["quest_giver_references"],
            "api_unavailable_or_failed"
        );
    }
    let (empty, lua) = capture("empty")?;
    assert_eq!(
        empty.sections["quest_giver_references"]["reported_quest_count"],
        0
    );
    assert_eq!(
        empty.sections["quest_giver_references"]["entries"],
        json!({})
    );
    assert_eq!(lua.globals().get::<u32>("reads")?, 0);
    let (secret, _) = capture("restricted")?;
    assert!(
        secret.sections["quest_giver_references"]["entries"]
            .as_object()
            .unwrap()
            .values()
            .all(|row| row == &json!({"status":"unavailable"}))
    );
    let (malformed, _) = capture("malformed")?;
    assert_eq!(
        malformed.sections["quest_giver_references"]["entries"]["1"],
        json!({"status":"reported"})
    );
    assert!(capture("combat").is_err());
    Ok(())
}
