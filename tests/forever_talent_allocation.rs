use azeroth_operator::import::decode;
use mlua::Lua;
use serde_json::json;

fn fixture(mode: &str) -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};actions=0
        active=7;nodeReads={};entryReads={};configReads=0;stageReads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return value=='PRIVATE' end
        function forbidden() actions=actions+1;error('forbidden action') end
        LoadAddOn=forbidden;UIParentLoadAddOn=forbidden;CastSpellByID=forbidden
        function copy(value)
            if type(value)~='table' then return value end
            local result={};for k,v in pairs(value) do result[k]=copy(v) end;return result
        end
        nodes={
            [11]={ID=11,activeEntry={entryID=111,rank=7},ranksPurchased=3,
                entryIDs={111,112},entryIDsWithCommittedRanks={111}},
            [12]={ID=12,activeEntry={entryID='PRIVATE',rank=0},ranksPurchased=0,
                entryIDs={121,122},entryIDsWithCommittedRanks={}}
        }
        C_ClassTalents={GetActiveConfigID=function() return active end,
            LoadConfig=forbidden,CommitConfig=forbidden,ImportLoadout=forbidden}
        C_Traits={PurchaseRank=forbidden,RefundRank=forbidden,GenerateImportString=forbidden}
        function C_Traits.GetConfigInfo(id)
            assert(id==7);configReads=configReads+1
            return {treeIDs={1},name='PRIVATE CUSTOM LOADOUT NAME'}
        end
        function C_Traits.GetTreeNodes(tree)
            assert(tree==1)
            if mode=='empty_graph' then return {} end
            if mode=='global_budget' or mode=='global_boundary' then
                local result={};for i=1,(mode=='global_budget' and 65 or 64) do result[i]=i end
                return result
            end
            return {11,12}
        end
        function C_Traits.GetNodeInfo(id,nodeID)
            assert(id==7);nodeReads[nodeID]=(nodeReads[nodeID] or 0)+1
            if mode=='global_budget' or mode=='global_boundary' then
                local entries={};for i=1,64 do entries[i]=nodeID*100+i end
                return {ID=nodeID,entryIDs=entries,ranksPurchased=0,entryIDsWithCommittedRanks={}}
            end
            local node=copy(nodes[nodeID])
            if mode=='legacy' then node.entryIDs=nil end
            if nodeID==11 then
                if mode=='empty_purchased' then node.entryIDs={} end
                if mode=='multiple_committed' then node.entryIDsWithCommittedRanks={111,112} end
                if mode=='wrong_committed' then node.entryIDsWithCommittedRanks={112} end
                if mode=='purchased_over_max' then node.ranksPurchased=5 end
            else
                if mode=='entries_missing' then node.entryIDs=nil end
                if mode=='entries_secret' then node.entryIDs='PRIVATE' end
                if mode=='entry_secret' then node.entryIDs={121,'PRIVATE'} end
                if mode=='entries_sparse' then node.entryIDs={[1]=121,[3]=122} end
                if mode=='entries_duplicate' then node.entryIDs={121,121} end
                if mode=='entries_global_duplicate' then node.entryIDs={111,122} end
                if mode=='entries_budget' then
                    node.entryIDs={};for i=1,65 do node.entryIDs[i]=120+i end
                end
                if mode=='committed_missing' then node.entryIDsWithCommittedRanks=nil end
                if mode=='committed_secret' then node.entryIDsWithCommittedRanks='PRIVATE' end
                if mode=='committed_duplicate' then node.entryIDsWithCommittedRanks={121,121} end
                if mode=='committed_sparse' then node.entryIDsWithCommittedRanks={[1]=121,[3]=122} end
                if mode=='committed_outside' then node.entryIDsWithCommittedRanks={999} end
                if mode=='committed_budget' then
                    node.entryIDsWithCommittedRanks={};for i=1,65 do node.entryIDsWithCommittedRanks[i]=120+i end
                end
                if mode=='purchased_secret' then node.ranksPurchased='PRIVATE' end
                if mode=='purchased_negative' then node.ranksPurchased=-1 end
                if mode=='purchased_fraction' then node.ranksPurchased=0.5 end
                if mode=='purchased_overflow' then node.ranksPurchased=4294967296 end
                if mode=='inactive_purchased' then node.ranksPurchased=1;node.entryIDsWithCommittedRanks={121} end
                if mode=='empty_node' then node.entryIDs={} end
                if mode=='empty_committed' then node.entryIDs={};node.entryIDsWithCommittedRanks={121} end
                if mode=='zero_with_committed' then node.entryIDsWithCommittedRanks={121,122} end
                if mode=='boosted_zero' then node.activeEntry={entryID=121,rank=2} end
                if mode=='boosted_entry_absent' then
                    node.activeEntry={entryID=121,rank=1};node.entryIDs={122}
                end
                if nodeReads[nodeID]==2 then
                    if mode=='inactive_base_transient' then node.activeEntry={entryID=121,rank=1} end
                    if mode=='inactive_id_transient' then node.ID=13 end
                    if mode=='inactive_unreadable' then return nil end
                    if mode=='entries_changed' then node.entryIDs={121} end
                    if mode=='purchased_changed' then node.ranksPurchased=1 end
                end
            end
            return node
        end
        function C_Traits.GetEntryInfo(id,entryID)
            assert(id==7);entryReads[entryID]=(entryReads[entryID] or 0)+1
            local entry={definitionID=entryID+100,maxRanks=entryID==111 and 4 or 2,
                name='PRIVATE SPELL NAME',description='PRIVATE'}
            if entryID==122 then
                if mode=='entry_nil' then return nil end
                if mode=='entry_error' then error('PRIVATE') end
                if mode=='entry_table_secret' then return 'PRIVATE' end
                if mode=='definition_nil' then entry.definitionID=nil end
                if mode=='definition_secret' then entry.definitionID='PRIVATE' end
                if mode=='definition_zero' then entry.definitionID=0 end
                if mode=='definition_fraction' then entry.definitionID=221.5 end
                if mode=='definition_duplicate' then entry.definitionID=211 end
                if mode=='definition_overflow' then entry.definitionID=4294967296 end
                if mode=='max_zero' then entry.maxRanks=0 end
                if mode=='max_secret' then entry.maxRanks='PRIVATE' end
                if mode=='max_infinite' then entry.maxRanks=math.huge end
                if mode=='max_fraction' then entry.maxRanks=1.5 end
                if mode=='subtree' then entry.subTreeID=9 end
                if mode=='subtree_secret' then entry.subTreeID='PRIVATE' end
                if entryReads[entryID]==2 then
                    if mode=='definition_changed' then entry.definitionID=223 end
                    if mode=='max_changed' then entry.maxRanks=3 end
                    if mode=='mutate_final_base' then nodes[12].activeEntry={entryID=121,rank=1} end
                end
                if mode=='getter_changed' then C_Traits.GetEntryInfo=function() return entry end end
                if mode=='config_changed' then active=8 end
                if mode=='family_changed' then AO.family='retail' end
                if mode=='combat_changed' then InCombatLockdown=function() return true end end
                if mode=='required_getter_changed' then C_Traits.GetNodeInfo=function() return nodes[11] end end
                if mode=='namespace_changed' then C_Traits=copy(C_Traits) end
            end
            return entry
        end
        function C_Traits.ConfigHasStagedChanges(id)
            assert(id==7);stageReads=stageReads+1
            if mode=='stage_mutates_final' and stageReads==4 then nodes[12].activeEntry={entryID=121,rank=1} end
            if (mode=='getter_between_passes' and stageReads==3) or
                (mode=='getter_final_stage' and stageReads==4) then
                local old=C_Traits.GetEntryInfo;C_Traits.GetEntryInfo=function(...) return old(...) end
            end
            return false
        end
        if mode=='missing_getter' then C_Traits.GetEntryInfo=nil end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load("AO.collectors={talents=AO.collectors.talents}")
        .exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua) -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
    lua.load("AO.Capture();assert(actions==0)").exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{export}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    Ok(snapshot)
}

#[test]
fn allocation_records_every_queried_entry_and_only_purchased_ranks()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture("normal")?;
    let saved = capture(&lua)?;
    let talents = &saved.sections["talents"];
    assert_eq!(
        talents["nodes"]["11"],
        json!({
            "entry_id":111,"rank":7,"tree_id":1,"definition_id":211,"purchased_rank":3,"max_rank":4
        })
    );
    assert!(talents["nodes"].get("12").is_none());
    assert_eq!(
        talents["definition_allocation"],
        json!({
            "coverage":"queried_config_tree_entries","queried_node_count":2,"queried_entry_count":4,
            "definitions":{
                "211":{"node_id":11,"entry_id":111,"tree_id":1,"purchased_rank":3,"max_rank":4},
                "212":{"node_id":11,"entry_id":112,"tree_id":1,"purchased_rank":0,"max_rank":2},
                "221":{"node_id":12,"entry_id":121,"tree_id":1,"purchased_rank":0,"max_rank":2},
                "222":{"node_id":12,"entry_id":122,"tree_id":1,"purchased_rank":0,"max_rank":2}
            }
        })
    );
    lua.load("assert(configReads==2 and stageReads==4);assert(nodeReads[11]==4 and nodeReads[12]==3);assert(entryReads[112]==2 and entryReads[122]==2)").exec()?;
    for mode in ["boosted_zero", "zero_with_committed"] {
        let saved = capture(&fixture(mode)?)?;
        assert_eq!(
            saved.sections["talents"]["definition_allocation"]["definitions"]["221"]["purchased_rank"],
            0,
            "{mode}"
        );
    }
    Ok(())
}

#[test]
fn allocation_is_optional_all_or_nothing_and_never_infers_unreadable_zero()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "legacy",
        "missing_getter",
        "entries_missing",
        "entries_secret",
        "entry_secret",
        "entries_sparse",
        "entries_duplicate",
        "entries_global_duplicate",
        "entries_budget",
        "committed_missing",
        "committed_secret",
        "committed_duplicate",
        "committed_sparse",
        "committed_outside",
        "committed_budget",
        "purchased_secret",
        "purchased_negative",
        "purchased_fraction",
        "purchased_overflow",
        "inactive_purchased",
        "empty_purchased",
        "empty_committed",
        "multiple_committed",
        "wrong_committed",
        "purchased_over_max",
        "entry_nil",
        "entry_error",
        "entry_table_secret",
        "definition_nil",
        "definition_secret",
        "definition_zero",
        "definition_fraction",
        "definition_duplicate",
        "definition_overflow",
        "max_zero",
        "max_secret",
        "max_infinite",
        "max_fraction",
        "subtree",
        "subtree_secret",
        "definition_changed",
        "max_changed",
        "getter_changed",
        "getter_between_passes",
        "getter_final_stage",
        "inactive_base_transient",
        "inactive_id_transient",
        "inactive_unreadable",
        "entries_changed",
        "purchased_changed",
        "boosted_entry_absent",
        "global_budget",
    ] {
        let saved = capture(&fixture(mode)?)?;
        let talents = saved
            .sections
            .get("talents")
            .unwrap_or_else(|| panic!("base lost: {mode}"));
        assert!(talents.get("definition_allocation").is_none(), "{mode}");
        if mode != "global_budget" {
            assert_eq!(talents["nodes"]["11"]["rank"], 7, "{mode}");
        }
        if mode == "legacy" {
            assert_eq!(talents["nodes"]["11"]["definition_id"], 211);
        }
    }
    Ok(())
}

#[test]
fn allocation_empty_and_maximum_coverage_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    for (mode, nodes, entries) in [
        ("empty_graph", 0, 0),
        ("empty_node", 2, 2),
        ("global_boundary", 64, 4096),
    ] {
        let saved = capture(&fixture(mode)?)?;
        let allocation = &saved.sections["talents"]["definition_allocation"];
        assert_eq!(allocation["queried_node_count"], nodes, "{mode}");
        assert_eq!(allocation["queried_entry_count"], entries, "{mode}");
        assert_eq!(
            allocation["definitions"].as_object().unwrap().len(),
            entries as usize,
            "{mode}"
        );
    }
    Ok(())
}

#[test]
fn final_base_graph_brackets_all_optional_allocation_getters()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "mutate_final_base",
        "stage_mutates_final",
        "config_changed",
        "required_getter_changed",
        "namespace_changed",
    ] {
        let saved = capture(&fixture(mode)?)?;
        assert!(!saved.sections.contains_key("talents"), "{mode}");
        assert!(saved.unavailable.contains_key("talents"), "{mode}");
    }
    for mode in ["combat_changed", "family_changed"] {
        fixture(mode)?
            .load("assert(not pcall(AO.collectors.talents));assert(actions==0)")
            .exec()?;
    }
    Ok(())
}
