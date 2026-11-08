use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn visible_gossip_capture_never_selects_quests() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0;visibleReads=0
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        GossipFrame={IsVisible=function()
            visibleReads=visibleReads+1
            if mode=='secret_visibility' then return 'PRIVATE' end
            return mode~='hidden' and not(mode=='closed' and visibleReads>1)
        end}
        C_GossipInfo={GetNumAvailableQuests=function()
            if mode=='secret_count' then return 'PRIVATE' end
            if mode=='empty' then return 0 end
            if mode=='mismatch' then return 3 end
            if mode=='count_changed' and reads>0 then return 3 end
            if mode=='oversize' then return 129 end
            return 2
        end,
        GetAvailableQuests=function()
            reads=reads+1
            if mode=='error' or mode=='both_error' or (mode=='second_error' and reads>1) then error('PRIVATE') end
            if mode=='nil_list' then return nil end
            if mode=='secret_list' then return 'PRIVATE' end
            if mode=='empty' then return {} end
            if mode=='map' then return {x={questID=2},y={questID=10}} end
            if mode=='sparse' then return {[1]={questID=2},[3]={questID=10}} end
            local a={questID=2,title='PRIVATE',repeatable=true,isTrivial=false,isIgnored=false}
            local b={questID=10,title='PRIVATE',repeatable=false}
            if mode=='secret_id' then a.questID='PRIVATE' end
            if mode=='bad_id' then a.questID=0 end
            if mode=='duplicate' then b.questID=2 end
            if mode=='changed' and reads>1 then a.questID=3 end
            if mode=='secret_flags' then a.repeatable='PRIVATE';a.isTrivial='PRIVATE' end
            if mode=='invalid_flags' then a.repeatable=1;a.isTrivial='no' end
            if mode=='changed_flags' and reads>1 then a.repeatable=false;a.isTrivial=true end
            if mode=='alias_only' then a.repeatable=nil;a.isRepeatable=true;b.repeatable=nil;b.isRepeatable=false end
            if mode=='alias_conflict' then a.isRepeatable=false end
            if mode=='alias_secret' then a.isRepeatable='PRIVATE' end
            if mode=='reordered' and reads>1 then return {b,a} end
            return {a,b}
        end,
        GetNumActiveQuests=function() return 2 end,
        GetActiveQuests=function()
            if mode=='both_error' then error('PRIVATE') end
            return {{questID=20,isComplete=true,title='PRIVATE'},{questID=21,isComplete=false}}
        end,
        SelectAvailableQuest=function() error('no selection') end,
        SelectActiveQuest=function() error('no selection') end,
        SelectOption=function() error('no selection') end,
        RefreshOptions=function() error('no refresh') end}
        function AcceptQuest() error('no acceptance') end
        function CompleteQuest() error('no turn-in') end
        function GetQuestReward() error('no reward') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads=0;visibleReads=0;AO.Capture()").exec()?;
        let value: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{value}\"").as_bytes(),
        )?)
    };
    let initial = capture("normal")?;
    assert_eq!(
        initial.sections["gossip_quests"]["available"]["2"]["repeatable"],
        true
    );
    assert_eq!(
        initial.sections["gossip_quests"]["active"]["20"]["is_complete"],
        true
    );
    assert!(!serde_json::to_string(&initial)?.contains("PRIVATE"));
    assert_eq!(
        capture("reordered")?.sections["gossip_quests"],
        initial.sections["gossip_quests"]
    );
    assert_eq!(
        capture("empty")?.sections["gossip_quests"]["available"],
        serde_json::json!({})
    );
    for mode in [
        "secret_count",
        "mismatch",
        "count_changed",
        "oversize",
        "error",
        "second_error",
        "nil_list",
        "secret_list",
        "map",
        "sparse",
        "secret_id",
        "bad_id",
        "duplicate",
        "changed",
    ] {
        let value = capture(mode)?;
        assert!(
            value.sections["gossip_quests"].get("available").is_none(),
            "{mode}"
        );
        assert!(
            value.sections["gossip_quests"].get("active").is_some(),
            "{mode}"
        );
    }
    for mode in ["secret_flags", "invalid_flags", "changed_flags"] {
        let value = capture(mode)?;
        let row = &value.sections["gossip_quests"]["available"]["2"];
        assert!(row.get("repeatable").is_none(), "{mode}");
        assert!(row.get("is_trivial").is_none(), "{mode}");
        assert_eq!(row["is_ignored"], false);
    }
    let aliases = capture("alias_only")?;
    assert_eq!(
        aliases.sections["gossip_quests"]["available"]["2"]["repeatable"],
        true
    );
    assert_eq!(
        aliases.sections["gossip_quests"]["available"]["10"]["repeatable"],
        false
    );
    for mode in ["alias_conflict", "alias_secret"] {
        assert!(
            capture(mode)?.sections["gossip_quests"]["available"]["2"]
                .get("repeatable")
                .is_none()
        );
    }
    for mode in ["both_error", "hidden", "closed", "secret_visibility"] {
        let value = capture(mode)?;
        assert!(!value.sections.contains_key("gossip_quests"), "{mode}");
        assert!(value.unavailable.contains_key("gossip_quests"), "{mode}");
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.load("C_GossipInfo.GetAvailableQuests=nil").exec()?;
    assert!(
        capture("normal")?.sections["gossip_quests"]
            .get("available")
            .is_none()
    );
    assert!(capture("combat").is_err());
    Ok(())
}
