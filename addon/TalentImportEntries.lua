-- Source-only conversion inspection. No import, purchase, selection or commit calls.
local _,AO=...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v),"Restricted talent entry")
    return v
end
local function num(v,low,high)
    assert(type(read(v))=="number" and v%1==0 and v>=low and v<=high,"Invalid talent entry number")
    return v
end
local function dense(t,limit)
    assert(type(read(t))=="table" and #t<=limit,"Invalid talent entry list")
    local n=0; for key in pairs(t) do num(key,1,#t); n=n+1 end
    assert(n==#t,"Sparse talent entry list"); return n
end
local function same(a,b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    for k,v in pairs(a) do if not same(v,b[k]) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function describe(plan)
    local result={}; local count=0
    for index,id in ipairs(plan.observation.nodes) do
        if plan.content[index].isNodeSelected then
            local node=read(C_Traits.GetNodeInfo(plan.observation.active_config_id,id))
            assert(type(node)=="table" and read(node.ID)==id,"Selected node unavailable")
            local d={type=num(node.type,0,255),max_ranks=num(node.maxRanks,1,65535),entries={}}
            local n=dense(node.entryIDs,256); assert(n>0,"Selected node has no entries")
            count=count+n; assert(count<=16384,"Too many selected entries")
            local seen={}
            for i,entryID in ipairs(node.entryIDs) do
                num(entryID,1,4294967295); assert(not seen[entryID],"Duplicate node entry"); seen[entryID]=true
                local entry=read(C_Traits.GetEntryInfo(plan.observation.active_config_id,entryID))
                assert(type(entry)=="table","Selected entry unavailable")
                d.entries[i]={id=entryID,max_ranks=num(entry.maxRanks,0,65535)}
            end
            local active=read(node.activeEntry)
            if active then
                assert(type(active)=="table","Invalid active entry")
                local entryID=num(active.entryID,1,4294967295)
                assert(seen[entryID],"Active entry outside node"); d.active_entry=entryID
            end
            result[tostring(id)]=d
        end
    end
    return result
end
function AO.PlanTalentImport(text,allowEmptyHash)
    local plan=AO.InspectTalentImport(text,allowEmptyHash)
    local frame=PlayerSpellsFrame.TalentsFrame
    assert(type(frame.ConvertToImportLoadoutEntryInfo)=="function" and Enum and Enum.TraitNodeType,"Native conversion unavailable")
    local tiered=num(Enum.TraitNodeType.Tiered,0,255)
    local selection=num(Enum.TraitNodeType.Selection,0,255)
    local subtree=num(Enum.TraitNodeType.SubTreeSelection,0,255)
    assert(tiered~=selection and tiered~=subtree and selection~=subtree,"Ambiguous node types")
    local descriptions=describe(plan); local expected={}
    local function add(id,entry,granted,purchased)
        expected[#expected+1]={nodeID=id,selectionEntryID=entry,ranksGranted=granted,ranksPurchased=purchased}
    end
    for index,id in ipairs(plan.observation.nodes) do
        local c=plan.content[index]; local d=descriptions[tostring(id)]
        if c.isNodeSelected then
            assert(not c.isNodeGranted or not c.isChoiceNode and not c.isPartiallyRanked,"Invalid granted node encoding")
            if not c.isNodeGranted then
                assert(c.isChoiceNode==(d.type==selection or d.type==subtree),"Choice node encoding mismatch")
            end
            local ranks=c.isNodeGranted and 0 or (c.isPartiallyRanked and c.partialRanksPurchased or d.max_ranks)
            assert(not c.isPartiallyRanked or ranks>0 and ranks<d.max_ranks,"Invalid partial ranks")
            if d.type==tiered then
                local remaining=ranks
                for i,entry in ipairs(d.entries) do
                    local used=math.min(remaining,entry.max_ranks)
                    local granted=c.isNodeGranted and i==1 and 1 or 0
                    if used>0 or granted>0 then add(id,entry.id,granted,used) end
                    remaining=remaining-used
                end
                assert(remaining==0,"Tiered ranks lost in conversion")
            else
                local entryID
                if c.isChoiceNode then
                    assert(d.entries[c.choiceNodeSelection],"Choice outside node")
                    entryID=d.entries[c.choiceNodeSelection].id
                else entryID=d.active_entry or d.entries[1].id end
                add(id,entryID,c.isNodeGranted and 1 or 0,ranks)
            end
        end
    end
    local converted=read(frame:ConvertToImportLoadoutEntryInfo(plan.observation.active_config_id,plan.observation.tree_id,plan.content))
    assert(dense(converted,16384)==#expected,"Native conversion lost or added entries")
    local copied={}
    for i,entry in ipairs(converted) do
        assert(type(read(entry))=="table","Invalid converted entry")
        local out={}; local keys=0
        for key,value in pairs(entry) do
            assert(key=='nodeID' or key=='selectionEntryID' or key=='ranksGranted' or key=='ranksPurchased',"Unexpected converted field")
            out[key]=num(value,0,4294967295); keys=keys+1
        end
        assert(keys==4 and same(out,expected[i]),"Native conversion differs from requested nodes")
        copied[i]=out
    end
    assert(PlayerSpellsFrame.TalentsFrame==frame and same(descriptions,describe(plan)),"Talent entries changed during conversion")
    assert(same(plan,AO.InspectTalentImport(text,allowEmptyHash)),"Talent import changed during conversion")
    plan.entries=copied; plan.node_descriptions=descriptions; plan.conversion_verified=true
    -- Eligibility, currencies and commit success still require native validation/readback.
    return plan
end
