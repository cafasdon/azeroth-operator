use azeroth_operator::import::decode;
use mlua::Lua;
use serde_json::json;

fn fixture(mode: &str) -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};actions=0
        active=7;configReads=0;treeReads={};nodeReads={};entryReads={};stageReads=0
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
            [11]={ID=11,activeEntry={entryID=111,rank=3},ranksPurchased=1,entryIDsWithCommittedRanks={111}},
            [12]={ID=12,activeEntry={entryID=112,rank=1},ranksPurchased=0,entryIDsWithCommittedRanks={}},
            [13]={ID=13,activeEntry={entryID=113,rank=1},ranksPurchased=0,entryIDsWithCommittedRanks={}},
            [14]={ID=14,activeEntry={entryID='PRIVATE',rank=0}},[15]={ID=15}
        }
        C_ClassTalents={GetActiveConfigID=function()
            if mode=='config_secret' then return 'PRIVATE' end
            if mode=='config_zero' then return 0 end
            return active
        end,LoadConfig=forbidden,CommitConfig=forbidden,ImportLoadout=forbidden}
        C_Traits={PurchaseRank=forbidden,RefundRank=forbidden,GenerateImportString=forbidden}
        function C_Traits.GetConfigInfo(id)
            assert(id==7);configReads=configReads+1
            if mode=='config_info_nil' then return nil end
            if mode=='config_info_secret' then return 'PRIVATE' end
            if mode=='tree_sparse' then return {treeIDs={[1]=1,[3]=2}} end
            if mode=='tree_duplicate' then return {treeIDs={1,1}} end
            if mode=='tree_secret' then return {treeIDs='PRIVATE'} end
            if mode=='tree_value_secret' then return {treeIDs={'PRIVATE'}} end
            if mode=='tree_zero' then return {treeIDs={0}} end
            if mode=='tree_empty' then return {treeIDs={}} end
            if mode=='tree_budget' then local ids={};for i=1,17 do ids[i]=i end;return {treeIDs=ids} end
            if mode=='tree_changed' and configReads>1 then return {treeIDs={1,3}} end
            return {treeIDs={1,2},name='PRIVATE CUSTOM LOADOUT NAME'}
        end
        function C_Traits.GetTreeNodes(tree)
            treeReads[tree]=(treeReads[tree] or 0)+1
            if mode=='empty_nodes' then return {} end
            if mode=='nodes_error' then error('PRIVATE') end
            if mode=='nodes_secret' then return 'PRIVATE' end
            if mode=='node_sparse' then return {[1]=11,[3]=12} end
            if mode=='node_duplicate' then return {11,11} end
            if mode=='cross_tree_duplicate' then return {11} end
            if mode=='node_budget' then
                if tree==2 then return {4097} end
                local ids={};for i=1,4096 do ids[i]=i end;return ids
            end
            if mode=='node_list_changed' and treeReads[tree]>1 then return tree==1 and {11,12,13} or {15} end
            return tree==1 and {11,12,13,14} or {15}
        end
        function C_Traits.GetNodeInfo(id,nodeID)
            assert(id==7);nodeReads[nodeID]=(nodeReads[nodeID] or 0)+1
            if mode=='node_budget' then return {ID=nodeID} end
            local node=copy(nodes[nodeID])
            if nodeID==11 then
                if mode=='node_nil' then return nil end
                if mode=='node_secret' then return 'PRIVATE' end
                if mode=='node_error' then error('PRIVATE') end
                if mode=='node_id_changed' then node.ID=99 end
                if mode=='node_id_secret' then node.ID='PRIVATE' end
                if mode=='entry_secret' then node.activeEntry='PRIVATE' end
                if mode=='rank_secret' then node.activeEntry.rank='PRIVATE' end
                if mode=='rank_negative' then node.activeEntry.rank=-1 end
                if mode=='rank_fraction' then node.activeEntry.rank=1.5 end
                if mode=='entry_id_secret' then node.activeEntry.entryID='PRIVATE' end
                if mode=='base_changed' and nodeReads[nodeID]>1 then node.activeEntry.rank=4 end
                if mode=='base_entry_changed' and nodeReads[nodeID]>1 then node.activeEntry.entryID=114 end
                if mode=='base_removed' and nodeReads[nodeID]>1 then node.activeEntry=nil end
                if mode=='purchased_secret' then node.ranksPurchased='PRIVATE' end
                if mode=='purchased_negative' then node.ranksPurchased=-1 end
                if mode=='purchased_over_max' then node.ranksPurchased=3 end
                if mode=='purchased_changed' and nodeReads[nodeID]>1 then node.ranksPurchased=2 end
                if mode=='committed_missing' then node.entryIDsWithCommittedRanks=nil end
                if mode=='committed_secret' then node.entryIDsWithCommittedRanks='PRIVATE' end
                if mode=='committed_ambiguous' then node.entryIDsWithCommittedRanks={111,114} end
                if mode=='committed_other' then node.entryIDsWithCommittedRanks={114} end
                if mode=='committed_duplicate' then node.entryIDsWithCommittedRanks={111,111} end
                if mode=='committed_sparse' then node.entryIDsWithCommittedRanks={[1]=111,[3]=114} end
                if mode=='committed_budget' then local ids={};for i=1,65 do ids[i]=i end;node.entryIDsWithCommittedRanks=ids end
            end
            return node
        end
        function C_Traits.GetEntryInfo(id,entry)
            assert(id==7 and entry>=111 and entry<=113)
            entryReads[entry]=(entryReads[entry] or 0)+1
            if entry==113 then return {maxRanks=1} end -- SubTreeSelection has no definition ID.
            if entry==112 then return {definitionID=212,maxRanks=1} end
            if mode=='definition_nil' then return nil end
            if mode=='definition_error' then error('PRIVATE') end
            if mode=='definition_secret_table' then return 'PRIVATE' end
            if mode=='definition_secret' then return {definitionID='PRIVATE',maxRanks=2} end
            if mode=='definition_zero' then return {definitionID=0,maxRanks=2} end
            if mode=='definition_fraction' then return {definitionID=211.5,maxRanks=2} end
            if mode=='max_zero' then return {definitionID=211,maxRanks=0} end
            if mode=='max_secret' then return {definitionID=211,maxRanks='PRIVATE'} end
            if mode=='max_infinite' then return {definitionID=211,maxRanks=math.huge} end
            if mode=='definition_changed' and entryReads[entry]>1 then return {definitionID=214,maxRanks=2} end
            if mode=='max_changed' and entryReads[entry]>1 then return {definitionID=211,maxRanks=3} end
            if mode=='optional_getter_changed' then
                C_Traits.GetEntryInfo=function() return {definitionID=211,maxRanks=2} end
            end
            if mode=='mutate_base_first' or (mode=='mutate_base_second' and entryReads[entry]>1) then
                nodes[15].activeEntry={entryID=115,rank=1}
            end
            if mode=='config_changed' then active=8 end
            if mode=='family_changed' then AO.family='retail' end
            if mode=='combat_changed' then InCombatLockdown=function() return true end end
            if mode=='required_getter_changed' then C_Traits.GetNodeInfo=function() return nodes[11] end end
            if mode=='namespace_changed' then C_Traits=copy(C_Traits) end
            return {definitionID=211,maxRanks=2}
        end
        function C_Traits.ConfigHasStagedChanges(id)
            assert(id==7);stageReads=stageReads+1
            if mode=='stage_mutates_final' and stageReads==4 then nodes[15].activeEntry={entryID=115,rank=1} end
            if mode=='stage_secret' then return 'PRIVATE' end
            if mode=='stage_error' then error('PRIVATE') end
            if mode=='stage_nil' then return nil end
            if mode=='stage_changed' then return stageReads>2 end
            if mode=='stage_getter_changed' then C_Traits.ConfigHasStagedChanges=function() return false end end
            return mode=='stage_true'
        end
        if mode=='missing_optional' then C_Traits.GetEntryInfo=nil end
        if mode=='missing_stage' then C_Traits.ConfigHasStagedChanges=nil end
        if mode=='missing_required' then C_Traits.GetNodeInfo=nil end
        if mode=='required_lookup_error' then
            C_Traits.GetNodeInfo=nil
            setmetatable(C_Traits,{__index=function(_,key) if key=='GetNodeInfo' then error('PRIVATE') end end})
        end
        if mode=='optional_lookup_error' then
            C_Traits.GetEntryInfo=nil
            setmetatable(C_Traits,{__index=function(_,key) if key=='GetEntryInfo' then error('PRIVATE') end end})
        end
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
    Ok(decode(
        format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
    )?)
}

#[test]
fn forever_native_definitions_keep_current_and_purchased_ranks_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["normal", "stage_true"] {
        let lua = fixture(mode)?;
        let saved = capture(&lua)?;
        assert_eq!(
            saved.sections["talents"],
            json!({
                "config_id":7,"observed_tree_ids":{"1":true,"2":true},"observed_node_count":5,
                "staged_changes_observed":mode=="stage_true",
                "loadout_export":{"status":"unavailable","reason":"client_not_supported"},
                "nodes":{
                    "11":{"entry_id":111,"rank":3,"tree_id":1,"definition_id":211,"purchased_rank":1,"max_rank":2},
                    "12":{"entry_id":112,"rank":1,"tree_id":1,"definition_id":212,"purchased_rank":0,"max_rank":1},
                    "13":{"entry_id":113,"rank":1,"tree_id":1}
                }
            }),
            "{mode}"
        );
        lua.load("assert(stageReads==4 and configReads==2);assert(nodeReads[11]==3 and nodeReads[14]==2);assert(actions==0)").exec()?;
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    let empty = capture(&fixture("empty_nodes")?)?;
    assert_eq!(empty.sections["talents"]["observed_node_count"], 0);
    assert_eq!(empty.sections["talents"]["nodes"], json!({}));
    Ok(())
}

#[test]
fn optional_definition_uncertainty_never_discards_a_stable_base_node()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "missing_optional",
        "optional_lookup_error",
        "definition_nil",
        "definition_error",
        "definition_secret_table",
        "definition_secret",
        "definition_zero",
        "definition_fraction",
        "max_zero",
        "max_secret",
        "max_infinite",
        "definition_changed",
        "max_changed",
        "optional_getter_changed",
        "purchased_secret",
        "purchased_negative",
        "purchased_over_max",
        "purchased_changed",
        "committed_missing",
        "committed_secret",
        "committed_ambiguous",
        "committed_other",
        "committed_duplicate",
        "committed_sparse",
        "committed_budget",
    ] {
        let saved = capture(&fixture(mode)?)?;
        assert_eq!(
            saved.sections["talents"]["nodes"]["11"],
            json!({"entry_id":111,"rank":3,"tree_id":1}),
            "{mode}"
        );
        assert_eq!(
            saved.sections["talents"]["observed_node_count"], 5,
            "{mode}"
        );
        assert!(
            !serde_json::to_string(&saved)?.contains("PRIVATE"),
            "{mode}"
        );
    }
    for mode in [
        "missing_stage",
        "stage_secret",
        "stage_error",
        "stage_nil",
        "stage_changed",
        "stage_getter_changed",
    ] {
        let saved = capture(&fixture(mode)?)?;
        assert!(
            saved.sections["talents"]
                .get("staged_changes_observed")
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            saved.sections["talents"]["nodes"]["11"]["definition_id"], 211,
            "{mode}"
        );
    }
    Ok(())
}

#[test]
fn incomplete_changed_or_invalid_base_graphs_fail_the_whole_section()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "missing_required",
        "required_lookup_error",
        "config_secret",
        "config_zero",
        "config_info_nil",
        "config_info_secret",
        "tree_sparse",
        "tree_duplicate",
        "tree_secret",
        "tree_value_secret",
        "tree_zero",
        "tree_empty",
        "tree_budget",
        "tree_changed",
        "nodes_error",
        "nodes_secret",
        "node_sparse",
        "node_duplicate",
        "cross_tree_duplicate",
        "node_budget",
        "node_list_changed",
        "node_nil",
        "node_secret",
        "node_error",
        "node_id_changed",
        "node_id_secret",
        "entry_secret",
        "rank_secret",
        "rank_negative",
        "rank_fraction",
        "entry_id_secret",
        "base_changed",
        "base_entry_changed",
        "base_removed",
        "mutate_base_first",
        "mutate_base_second",
        "stage_mutates_final",
        "config_changed",
        "required_getter_changed",
        "namespace_changed",
    ] {
        let lua = fixture(mode)?;
        let saved = capture(&lua)?;
        assert!(!saved.sections.contains_key("talents"), "{mode}");
        assert!(saved.unavailable.contains_key("talents"), "{mode}");
        assert!(
            !serde_json::to_string(&saved)?.contains("PRIVATE"),
            "{mode}"
        );
    }
    for mode in ["family_changed", "combat_changed"] {
        let lua = fixture(mode)?;
        lua.load("assert(not pcall(AO.collectors.talents));assert(actions==0)")
            .exec()?;
    }
    let lua = fixture("normal")?;
    lua.load("InCombatLockdown=function() return true end;assert(not pcall(AO.collectors.talents));assert(not pcall(AO.Capture));assert(actions==0)").exec()?;
    Ok(())
}
