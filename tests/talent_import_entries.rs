use mlua::Lua;

#[test]
fn import_entries_preserve_choices_grants_and_tiered_ranks() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={}; Enum={TraitNodeType={Single=0,Selection=2,SubTreeSelection=3,Tiered=4}}
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        function reset()
            mode=nil; reads=0
            plan={observation={active_config_id=7,tree_id=100,nodes={10,20,30,40,50}},content={},game_execution=false,semantic_legality_verified=false}
            for i=1,5 do plan.content[i]={isNodeSelected=i~=5,isNodeGranted=false,isPartiallyRanked=false,isChoiceNode=false,partialRanksPurchased=0,choiceNodeSelection=1} end
            plan.content[1].isPartiallyRanked=true; plan.content[1].partialRanksPurchased=1
            plan.content[2].isChoiceNode=true; plan.content[2].choiceNodeSelection=2
            plan.content[3].isPartiallyRanked=true; plan.content[3].partialRanksPurchased=2
            plan.content[4].isNodeGranted=true
            nodes={ [10]={ID=10,type=0,maxRanks=2,entryIDs={101}},[20]={ID=20,type=2,maxRanks=1,entryIDs={201,202}},
                [30]={ID=30,type=4,maxRanks=3,entryIDs={301,302}},[40]={ID=40,type=4,maxRanks=2,entryIDs={401,402}} }
            ranks={[101]=2,[201]=1,[202]=1,[301]=1,[302]=2,[401]=1,[402]=1}
            expected={{nodeID=10,selectionEntryID=101,ranksGranted=0,ranksPurchased=1},
                {nodeID=20,selectionEntryID=202,ranksGranted=0,ranksPurchased=1},
                {nodeID=30,selectionEntryID=301,ranksGranted=0,ranksPurchased=1},
                {nodeID=30,selectionEntryID=302,ranksGranted=0,ranksPurchased=1},
                {nodeID=40,selectionEntryID=401,ranksGranted=1,ranksPurchased=0}}
            function AO.InspectTalentImport() return copy(plan) end
            C_Traits={GetNodeInfo=function(_,id)
                reads=reads+1; local n=nodes[id]
                if mode=='race' and reads>4 and id==10 then n=copy(n); n.maxRanks=3 end
                return n
            end,GetEntryInfo=function(_,id) if ranks[id] then return {maxRanks=ranks[id]} end end}
            frame={ConvertToImportLoadoutEntryInfo=function(_,active,tree,content)
                assert(active==7 and tree==100)
                local r=copy(expected)
                if mode=='drop' then table.remove(r,1) elseif mode=='extra' then r[#r+1]=copy(r[1])
                elseif mode=='wrong_rank' then r[3].ranksPurchased=2 elseif mode=='wrong_choice' then r[2].selectionEntryID=201
                elseif mode=='unknown_field' then r[1].other=1 elseif mode=='sparse' then r[2]=nil
                elseif mode=='mutate_content' then content[1].partialRanksPurchased=2
                elseif mode=='spec' then plan.observation.tree_id=200 end
                return r
            end}
            PlayerSpellsFrame={TalentsFrame=frame}
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/TalentImportEntries.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        local result=AO.PlanTalentImport('synthetic')
        assert(result.conversion_verified and not result.game_execution and not result.semantic_legality_verified)
        assert(#result.entries==5 and result.entries[2].selectionEntryID==202)
        assert(result.entries[3].ranksPurchased==1 and result.entries[4].ranksPurchased==1)
        assert(result.entries[5].ranksGranted==1)
        result.entries[1].ranksPurchased=99; assert(expected[1].ranksPurchased==1)
        reset(); nodes[20].type=3; assert(AO.PlanTalentImport('synthetic').conversion_verified)
        reset(); plan.content[1].isPartiallyRanked=false; expected[1].ranksPurchased=2
        assert(AO.PlanTalentImport('synthetic').entries[1].ranksPurchased==2)
        reset(); nodes[10].activeEntry={entryID=101}; assert(AO.PlanTalentImport('synthetic').conversion_verified)
        for _,failure in ipairs({'drop','extra','wrong_rank','wrong_choice','unknown_field','sparse','race','mutate_content','spec'}) do
            reset(); mode=failure; assert(not pcall(AO.PlanTalentImport,'synthetic'),failure)
        end
        reset(); nodes[20]=nil; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); ranks[302]=nil; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); ranks[302]=0; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); nodes[20].entryIDs={201,201}; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); nodes[10].activeEntry={entryID=999}; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); plan.content[2].choiceNodeSelection=3; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); plan.content[1].partialRanksPurchased=2; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); plan.content[2].isChoiceNode=false; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); plan.content[4].isChoiceNode=true; assert(not pcall(AO.PlanTalentImport,'synthetic'))
        reset(); function issecretvalue(v) return v==302 end
        assert(not pcall(AO.PlanTalentImport,'synthetic')); issecretvalue=nil
        reset(); frame.ConvertToImportLoadoutEntryInfo=nil; assert(not pcall(AO.PlanTalentImport,'synthetic'))
    "#).exec()
}
