-- Fixed data fields only. No Lua evaluation, network input or automatic execution.
local _, AO = ...
local statuses={applied=true,outcome_unknown=true,copied_source_removal_unconfirmed=true,already_selected=true}
local kinds={create=true,edit=true,delete=true,move=true,set_binding=true,equip_bag_item=true,move_bag_item=true,split_bag_item=true,load_talent_config=true,create_talent_config=true,import_talent_config=true,rename_talent_config=true,delete_talent_config=true,move_action_slot=true,copy_action_slot=true,modify_equipment_set=true,delete_equipment_set=true,use_equipment_set=true,assign_equipment_set_spec=true,unassign_equipment_set_spec=true,create_equipment_set=true,save_equipment_set=true}
local executors={create="ApplyMacroOperation",edit="ApplyMacroOperation",delete="ApplyMacroOperation",move="ApplyMacroOperation",
    set_binding="ApplyBindingOperation",equip_bag_item="ApplyEquipmentOperation",move_bag_item="ApplyBagOperation",split_bag_item="ApplyBagOperation",
    load_talent_config="ApplyTalentOperation",create_talent_config="ApplyTalentCreate",import_talent_config="ApplyTalentImport",rename_talent_config="ApplyTalentMetadata",delete_talent_config="ApplyTalentMetadata",move_action_slot="ApplyActionOperation",copy_action_slot="ApplyActionOperation",
    modify_equipment_set="ApplyEquipmentSetOperation",delete_equipment_set="ApplyEquipmentSetOperation",use_equipment_set="ApplyEquipmentSetUse",assign_equipment_set_spec="ApplyEquipmentSetSpec",unassign_equipment_set_spec="ApplyEquipmentSetSpec",create_equipment_set="ApplyEquipmentSetSave",save_equipment_set="ApplyEquipmentSetSave"}
local verifiers={load_talent_config="VerifyTalentOperation",create_talent_config="VerifyTalentCreate",import_talent_config="VerifyTalentImport",rename_talent_config="VerifyTalentMetadata",delete_talent_config="VerifyTalentMetadata",move_action_slot="VerifyActionOperation",copy_action_slot="VerifyActionOperation",
    move_bag_item="VerifyBagOperation",split_bag_item="VerifyBagOperation",
    modify_equipment_set="VerifyEquipmentSetOperation",delete_equipment_set="VerifyEquipmentSetOperation",use_equipment_set="VerifyEquipmentSetUse",assign_equipment_set_spec="VerifyEquipmentSetSpec",unassign_equipment_set_spec="VerifyEquipmentSetSpec",create_equipment_set="VerifyEquipmentSetSave",save_equipment_set="VerifyEquipmentSetSave"}
local function readReceipts()
    local saved=AzerothOperatorOperations or ""
    assert(type(saved)=="string" and #saved<=16384, "Invalid receipt checkpoint")
    local receipts,count={},0
    for row in saved:gmatch("[^;]+") do
        local id,status,kind=row:match("^([0-9a-f]+):([a-z_]+):([a-z_]+)$")
        assert(id and #id==64 and statuses[status] and kinds[kind] and not receipts[id], "Invalid receipt entry")
        receipts[id]={status=status,kind=kind}; count=count+1
    end
    assert(count<=128 and (saved=="" or (saved:sub(-1)==";" and not saved:find(";;") and saved:sub(1,1)~=";")), "Invalid receipt checkpoint")
    return receipts,count
end
local function saveReceipts(receipts)
    local ids,rows={},{}
    for id in pairs(receipts) do ids[#ids+1]=id end
    table.sort(ids)
    for _,id in ipairs(ids) do
        local r=receipts[id]
        rows[#rows+1]=id..":"..r.status..":"..r.kind..";"
    end
    AzerothOperatorOperations=table.concat(rows)
end
local pendingObservations={}
function AO.ReadMacroReceipts()
    local receipts,count=readReceipts()
    local changed=false
    for id,pending in pairs(pendingObservations) do
        if GetTime()>pending.deadline then pendingObservations[id]=nil
        elseif receipts[id] and receipts[id].status=="outcome_unknown" then
            local ok,verified=pcall(pending.verify,pending.operation)
            if ok and verified then
                receipts[id]={kind=pending.operation.kind,status="applied"}
                pendingObservations[id]=nil; changed=true
            end
        else pendingObservations[id]=nil end
    end
    if changed then saveReceipts(receipts) end
    return receipts,count
end
local function unhex(value)
    if value=="-" then return "" end
    assert(#value%2==0 and not value:find("[^0-9a-f]"), "Invalid encoded text")
    return (value:gsub("..",function(pair) return string.char(tonumber(pair,16)) end))
end
local function integer(value)
    assert(value:match("^[1-9][0-9]*$") and #value<=10, "Invalid integer")
    local result=tonumber(value)
    assert(result<=4294967295, "Integer out of range")
    return result
end
function AO.DecodeMacroOperation(text)
    assert(type(text)=="string" and #text<=8192, "Operation too large")
    local fields={}
    local separator=text:sub(1,11)=="AO-MACRO-1:" and ":" or "|"
    local pattern=separator==":" and "([^:]*):" or "([^|]*)|"
    for field in (text..separator):gmatch(pattern) do fields[#fields+1]=field end
    assert(#fields==18 and fields[1]=="AO-MACRO-1", "Invalid operation format")
    for _,index in ipairs({2,3}) do
        assert(#fields[index]==64 and not fields[index]:find("[^0-9a-f]"), "Invalid operation identity")
    end
    assert(fields[4]=="retail", "Unsupported family")
    local operation={id=fields[2],snapshot_id=fields[3],family=fields[4],
        variant=unhex(fields[5]),build=unhex(fields[6]),interface=integer(fields[7]),
        character=unhex(fields[8]),realm=unhex(fields[9]),kind=fields[10]}
    assert(operation.kind=="create" or operation.kind=="edit" or operation.kind=="delete" or operation.kind=="move", "Unsupported operation")
    operation.source_scope=fields[11]~="-" and fields[11] or nil
    operation.target_scope=fields[12]~="-" and fields[12] or nil
    if operation.kind~="create" then
        operation.expected={name=unhex(fields[13]),icon=integer(fields[14]),body=unhex(fields[15])}
    else assert(fields[11]=="-" and fields[13]=="-" and fields[14]=="-" and fields[15]=="-", "Unexpected source fields") end
    if operation.kind=="create" or operation.kind=="edit" then
        operation.desired={name=unhex(fields[16]),icon=integer(fields[17]),body=unhex(fields[18])}
    else assert(fields[16]=="-" and fields[17]=="-" and fields[18]=="-", "Unexpected desired fields") end
    if operation.kind=="edit" or operation.kind=="delete" then assert(fields[12]=="-", "Unexpected target scope") end
    return operation
end

-- Keep the exact reviewed bytes private to this module. Editing the input invalidates
-- the review; an Apply event cannot silently substitute different operation data.
local reviewedText
local function decodeOperation(text)
    if type(text)=="string" and text:sub(1,15)=="AO-TALIMPORT-1:" then
        assert(AO.DecodeTalentImport,"Talent import unavailable")
        return AO.DecodeTalentImport(text)
    end
    if type(text)=="string" and text:sub(1,15)=="AO-TALCREATE-1:" then
        assert(AO.DecodeTalentCreate,"Talent creation import unavailable")
        return AO.DecodeTalentCreate(text)
    end
    if type(text)=="string" and text:sub(1,13)=="AO-TALMETA-1:" then
        assert(AO.DecodeTalentMetadata,"Talent metadata import unavailable")
        return AO.DecodeTalentMetadata(text)
    end
    if type(text)=="string" and text:sub(1,13)=="AO-SETSAVE-1:" then
        assert(AO.DecodeEquipmentSetSave,"Equipment set save import unavailable")
        return AO.DecodeEquipmentSetSave(text)
    end
    if type(text)=="string" and text:sub(1,13)=="AO-SETSPEC-1:" then
        assert(AO.DecodeEquipmentSetSpec,"Equipment set association import unavailable")
        return AO.DecodeEquipmentSetSpec(text)
    end
    if type(text)=="string" and (text:sub(1,9)=="AO-SET-1:" or text:sub(1,13)=="AO-SET-USE-1:") then
        assert(AO.DecodeEquipmentSetOperation,"Equipment set import unavailable")
        return AO.DecodeEquipmentSetOperation(text)
    end
    if type(text)=="string" and (text:sub(1,12)=="AO-ACTION-1:" or text:sub(1,16)=="AO-ACTIONCOPY-1:") then
        assert(AO.DecodeActionOperation,"Action import unavailable")
        return AO.DecodeActionOperation(text)
    end
    if type(text)=="string" and text:sub(1,12)=="AO-TALENT-1:" then
        assert(AO.DecodeTalentOperation,"Talent import unavailable")
        return AO.DecodeTalentOperation(text)
    end
    if type(text)=="string" and (text:sub(1,9)=="AO-BAG-1:" or text:sub(1,11)=="AO-SPLIT-1:") then
        assert(AO.DecodeBagOperation,"Bag import unavailable")
        return AO.DecodeBagOperation(text)
    end
    if type(text)=="string" and text:sub(1,11)=="AO-EQUIP-1:" then
        assert(AO.DecodeEquipmentOperation,"Equipment import unavailable")
        return AO.DecodeEquipmentOperation(text)
    end
    if type(text)=="string" and text:sub(1,10)=="AO-BIND-1:" then
        assert(AO.DecodeBindingOperation, "Binding import unavailable")
        return AO.DecodeBindingOperation(text)
    end
    return AO.DecodeMacroOperation(text)
end
function AO.InvalidateMacroReview()
    reviewedText=nil
end
function AO.ReviewMacroImport(text)
    reviewedText=nil
    local operation=decodeOperation(text)
    reviewedText=text
    return operation
end
function AO.ApplyReviewedMacro(text)
    assert(reviewedText and text==reviewedText, "Review the current operation before applying")
    reviewedText=nil
    return AO.ApplyImportedMacro(text)
end

-- The review UI must call this only from the player's explicit Apply action.
function AO.ApplyImportedMacro(text)
    local operation=decodeOperation(text)
    local _,build,_,interface=GetBuildInfo()
    assert(operation.family==AO.family and operation.variant==AO.variant and operation.build==tostring(build)
        and operation.interface==interface, "Operation targets a different client")
    assert(operation.character==UnitName("player") and operation.realm==GetRealmName(), "Operation targets a different character")
    assert(not InCombatLockdown(), "Operation unavailable in combat")
    local receipts,count=readReceipts()
    assert(not receipts[operation.id], "Operation already attempted; reconcile before retry")
    assert(count<128, "Operation receipt limit reached")
    receipts[operation.id]={status="outcome_unknown",kind=operation.kind}
    saveReceipts(receipts)
    local executor=AO[executors[operation.kind]]
    local ok,result=pcall(executor,operation)
    if ok and (type(result)~="table" or result.kind~=operation.kind or not statuses[result.status]) then
        ok=false; result="Invalid operation result; outcome remains unknown"
    end
    if ok then receipts[operation.id]=result end
    saveReceipts(receipts)
    local verify=verifiers[operation.kind] and AO[verifiers[operation.kind]]
    if verify and ok and result.status=="outcome_unknown" then
        pendingObservations[operation.id]={operation=operation,verify=verify,deadline=GetTime()+60}
    end
    -- SavedVariables are persisted only at reload/logout; never promise crash durability.
    return receipts[operation.id],ok,not ok and type(result)=="string" and result or nil
end
