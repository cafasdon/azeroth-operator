-- Source-only read-only import inspection. Never imports, stages or commits talents.
local _,AO=...
local function read(v)
    assert(not issecretvalue or not issecretvalue(v), "Restricted import observation")
    return v
end
local function integer(v,low,high)
    assert(type(read(v))=="number" and v%1==0 and v>=low and v<=high, "Invalid import number")
    return v
end
local function dense(t,maximum)
    assert(type(read(t))=="table" and #t<=maximum, "Invalid import list")
    local n=0
    for k in pairs(t) do n=n+1; integer(k,1,#t) end
    assert(n==#t,"Sparse import list"); return n
end
local function hash(t)
    assert(dense(t,16)==16,"Invalid tree hash")
    local copy={}; for i=1,16 do copy[i]=integer(t[i],0,255) end
    return copy
end
local function same(a,b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    for k,v in pairs(a) do if not same(v,b[k]) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function observe(frame)
    assert(read(frame:IsShown())==true and read(frame:IsInspecting())==false and read(frame:IsCommitInProgress())==false,"Open idle player Talents")
    local active=integer(C_ClassTalents.GetActiveConfigID(),1,4294967295)
    assert(frame:GetConfigID()==active and read(C_Traits.ConfigHasStagedChanges(active))==false,"Working talents changed")
    local spec=integer(C_SpecializationInfo.GetSpecializationInfo(C_SpecializationInfo.GetSpecialization()),1,65535)
    local info=read(frame:GetTreeInfo()); assert(type(info)=="table","Tree unavailable")
    local tree=integer(info.ID,1,4294967295)
    local ids=read(C_Traits.GetTreeNodes(tree)); local n=dense(ids,4096); assert(n>0,"Empty talent tree")
    local nodes={}; local previous=0
    for i=1,n do nodes[i]=integer(ids[i],previous+1,4294967295); previous=nodes[i] end
    return {active_config_id=active,spec_id=spec,tree_id=tree,nodes=nodes,tree_hash=hash(C_Traits.GetTreeHash(tree)),serialization_version=integer(C_Traits.GetLoadoutSerializationVersion(),0,255)}
end
function AO.InspectTalentImport(text,allowEmptyHash)
    assert(AO.family=="retail" and type(text)=="string" and #text>0 and #text<=16384 and not text:find("[^A-Za-z0-9+/]"),"Invalid talent import text")
    assert(allowEmptyHash==nil or type(allowEmptyHash)=="boolean","Invalid empty-hash choice")
    local frame=PlayerSpellsFrame and PlayerSpellsFrame.TalentsFrame
    assert(frame and ExportUtil and ExportUtil.MakeImportDataStream,"Native talent parser unavailable")
    local before=observe(frame)
    local stream=ExportUtil.MakeImportDataStream(text)
    local total=integer(stream:GetNumberOfBits(),152,98304)
    assert(total==#text*6,"Invalid import stream length")
    local used=0; local bounded={}
    function bounded:GetNumberOfBits() return total end
    function bounded:ExtractValue(width)
        integer(width,1,32); assert(used+width<=total,"Truncated talent import")
        local value=integer(stream:ExtractValue(width),0,2^width-1); used=used+width; return value
    end
    local valid,version,spec,encodedHash=frame:ReadLoadoutHeader(bounded)
    assert(read(valid)==true and used==152 and version==before.serialization_version and spec==before.spec_id,"Talent import header mismatch")
    encodedHash=hash(encodedHash)
    local empty=true; for i=1,16 do if encodedHash[i]~=0 then empty=false end end
    assert(empty and allowEmptyHash==true or not empty and same(encodedHash,before.tree_hash),"Talent tree hash mismatch or omitted")
    local content=read(frame:ReadLoadoutContent(bounded,before.tree_id))
    assert(dense(content,4096)==#before.nodes,"Incomplete talent content")
    local copied={}
    for i,node in ipairs(content) do
        assert(type(read(node))=="table","Invalid talent node")
        local out={}
        for _,key in ipairs({'isNodeSelected','isNodeGranted','isPartiallyRanked','isChoiceNode'}) do
            assert(type(read(node[key]))=="boolean","Invalid node flag"); out[key]=node[key]
        end
        out.partialRanksPurchased=integer(node.partialRanksPurchased,0,63)
        out.choiceNodeSelection=integer(node.choiceNodeSelection,1,4)
        assert(not out.isPartiallyRanked or out.partialRanksPurchased>0,"Zero partial rank")
        copied[i]=out
    end
    assert(total-used<=5,"Extra talent import data")
    if total>used then assert(bounded:ExtractValue(total-used)==0,"Nonzero import padding") end
    assert(same(before,observe(frame)),"Talent tree changed during inspection")
    return {observation=before,content=copied,import_string=text,hash_omitted=empty,semantic_legality_verified=false,game_execution=false}
end

-- Compare parsed node semantics, not text bytes: requested imports may omit the
-- hash, while native exports contain it. The captured tree identity must agree.
-- This is read-only evidence for a future executor, not an import success receipt.
function AO.TalentImportContentsMatch(expected,text)
    assert(type(read(expected))=="table" and type(read(expected.observation))=="table"
        and type(read(expected.content))=="table","Captured import inspection required")
    local actual=AO.InspectTalentImport(text,false)
    if not same(expected.observation,actual.observation) or not same(expected.content,actual.content) then return false end
    local again=AO.InspectTalentImport(text,false)
    return same(actual,again)
end
