use mlua::Lua;

#[test]
fn import_waits_for_requested_active_and_saved_contents() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105}
        function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        function GetBuildInfo() return '', '70688', '',120105 end
        function UnitName() return 'Fixture' end; function GetRealmName() return 'Synthetic' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end; function UnitChannelInfo() end
        function GetCursorInfo() return cursor end; function StaticPopup_Visible() return popup end
        function finish()
            current='TkVX'
            entries['11']={name='New',uses_shared_action_bars=false,import_string=current,serialization_version=2}
            if mode~='unselected' then selected=11; frame.lastSelectedConfigID=11; uiSelection=11 end
            if mode=='wrong_content' then entries['11'].import_string='Tk8=' end
            if mode=='wrong_shared' then entries['11'].uses_shared_action_bars=true end
            if mode=='extra' then entries['12']=copy(entries['11']) end
            if mode=='bystander' then entries['8'].name='Changed' end
            if mode=='active' then current='Tk8=' end
        end
        function reset()
            mode=nil; writes=0; starts=0; selected=8; current='QUJDRA=='; combat=false; casting=nil; cursor=nil; popup=nil; staged=false
            visible=true; inspect=false; busy=false; canCreate=true
            entries={['8']={name='Old',uses_shared_action_bars=true,import_string='RUZHSA==',serialization_version=2}}
            op={kind='import_talent_config',family='retail',variant='live',build='70688',interface=120105,character='Fixture',realm='Synthetic',name='New',import_string='AAAA',allow_empty_hash=false,expected_new_shared_action_bars=false,
                expected={spec_id=263,active_config_id=7,selected_config_id=8,serialization_version=2,active_import_string=current,entries=copy(entries)}}
            C_SpecializationInfo={GetSpecialization=function() return 1 end,GetSpecializationInfo=function() return 263 end}
            C_ClassTalents={GetActiveConfigID=function() return 7 end,GetLastSelectedSavedConfigID=function() return selected end,
                GetConfigIDsBySpecID=function() local ids={} for id in pairs(entries) do ids[#ids+1]=tonumber(id) end return ids end,
                CanCreateNewConfig=function() return canCreate end,CanEditTalents=function() return true end,
                ImportLoadout=function(id,nodes,name,text)
                    assert(id==7 and text=='AAAA' and nodes[1].nodeID==10)
                    nodes[1].nodeID=999 -- native input mutation must not alter retained plan
                    assert(name=='New'); writes=writes+1
                    if checkReceipt then assert(AzerothOperatorOperations==op.id..':outcome_unknown:import_talent_config;') end
                    if mode=='throw_before' then error('before') end
                    if mode~='delay' and mode~='reject' then finish() end
                    if mode=='throw_after' then error('after') end
                    return mode~='false_effect' and mode~='reject'
                end}
            C_Traits={GetLoadoutSerializationVersion=function() return 2 end,ConfigHasStagedChanges=function() return staged end,
                GenerateImportString=function(id) return id==7 and current or entries[tostring(id)].import_string end,
                GetConfigInfo=function(id) local e=entries[tostring(id)]; return {name=e.name,usesSharedActionBars=e.uses_shared_action_bars} end}
            frame={IsShown=function() return visible end,IsInspecting=function() return inspect end,IsCommitInProgress=function() return busy end,
                GetConfigID=function() return 7 end,HasAnyPurchasedRanks=function() return true end,
                OnTraitConfigCreateStarted=function(_,purchased)
                    assert(purchased); starts=starts+1; if mode=='callback_error' then error('callback') end
                end}
            uiSelection=8; frame.lastSelectedConfigID=8
            frame.LoadSystem={GetSelectionID=function() return uiSelection end}
            PlayerSpellsFrame={TalentsFrame=frame}
            function AO.PlanTalentImport(text,allow)
                assert(text=='AAAA' and allow==false)
                return {conversion_verified=true,observation={active_config_id=7,spec_id=263,serialization_version=2},entries={{nodeID=10}},content='TkVX'}
            end
            function AO.TalentImportContentsMatch(plan,text)
                assert(plan.entries[1].nodeID==10)
                return text==plan.content
            end
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    for source in [
        include_str!("../addon/TalentMetadata.lua"),
        include_str!("../addon/TalentImportExecute.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.load(r#"
        assert(AO.ApplyTalentImport(op).status=='applied' and writes==1 and starts==1)
        assert(not pcall(AO.ApplyTalentImport,op) and writes==1)
        reset(); mode='false_effect'; assert(AO.ApplyTalentImport(op).status=='applied' and starts==0)
        for _,failure in ipairs({'delay','reject','unselected','wrong_content','wrong_shared','extra','bystander','active'}) do
            reset(); mode=failure; assert(AO.ApplyTalentImport(op).status=='outcome_unknown' and writes==1,failure)
        end
        reset(); mode='delay'; assert(AO.ApplyTalentImport(op).status=='outcome_unknown'); finish()
        assert(AO.VerifyTalentImport(op) and writes==1 and starts==1)
        for _,failure in ipairs({'throw_before','throw_after','callback_error'}) do
            reset(); mode=failure; assert(not pcall(AO.ApplyTalentImport,op) and writes==1,failure)
        end
        for _,failure in ipairs({'combat','casting','cursor','popup','hidden','inspect','busy','capacity','staged','stale','collision','callback','api'}) do
            reset()
            if failure=='combat' then combat=true elseif failure=='casting' then casting='spell' elseif failure=='cursor' then cursor='item'
            elseif failure=='popup' then popup=true elseif failure=='hidden' then visible=false elseif failure=='inspect' then inspect=true
            elseif failure=='busy' then busy=true elseif failure=='capacity' then canCreate=false elseif failure=='staged' then staged=true
            elseif failure=='stale' then current='Tk8=' elseif failure=='collision' then op.name='OLD'
            elseif failure=='callback' then frame.OnTraitConfigCreateStarted=nil elseif failure=='api' then C_ClassTalents.ImportLoadout=nil end
            assert(not pcall(AO.ApplyTalentImport,op) and writes==0,failure)
        end
        reset(); op.expected_new_shared_action_bars=nil; assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
        reset(); frame.autoLoadNewConfigID=99; assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
        reset(); frame.nextNewConfigRequiresPopulatedCheck=true; assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
        reset(); frame.areClassTalentCommitVisualsActive=true; assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
        reset(); function issecretvalue(v) return v==true end
        assert(not pcall(AO.ApplyTalentImport,op) and writes==0); issecretvalue=nil
        reset(); AO.family='forever'; assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
    "#).exec()?;
    lua.load(
        r#"
        reset(); AO.family='retail'; assert(not AO.VerifyTalentImport(op))
        mode='delay'; assert(AO.ApplyTalentImport(op).status=='outcome_unknown')
        assert(not pcall(AO.ApplyTalentImport,op) and writes==1)
        finish(); frame.lastSelectedConfigID=8; assert(not AO.VerifyTalentImport(op))
        frame.lastSelectedConfigID=11; uiSelection=8; assert(not AO.VerifyTalentImport(op))
        uiSelection=11; assert(AO.VerifyTalentImport(op))
        op.name='Changed'; assert(not AO.VerifyTalentImport(op))
        reset(); AO.PlanTalentImport=function() error('invalid import') end
        assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
    "#,
    )
    .exec()?;
    // Compose all actual addon helpers; only Blizzard APIs/streams remain fixtures.
    for source in [
        include_str!("../addon/TalentImportValidation.lua"),
        include_str!("../addon/TalentImportEntries.lua"),
    ] {
        lua.load(source)
            .call::<()>(("AzerothOperator", ao.clone()))?;
    }
    lua.globals().set("requested", import_text(true, 1))?;
    lua.globals().set("nativeDesired", import_text(false, 1))?;
    lua.globals().set("nativeOther", import_text(false, 2))?;
    lua.load(r#"
        local actualPlan,actualMatch=AO.PlanTalentImport,AO.TalentImportContentsMatch
        local alphabet='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
        ExportUtil={MakeImportDataStream=function(text)
            local position=0
            return {GetNumberOfBits=function() return #text*6 end,ExtractValue=function(_,width)
                local value=0
                for bit=0,width-1 do
                    local index=math.floor(position/6)+1
                    local digit=assert(alphabet:find(text:sub(index,index),1,true))-1
                    value=value+math.floor(digit/2^(position%6))%2*2^bit; position=position+1
                end
                return value
            end}
        end}
        Enum={TraitNodeType={Single=0,Selection=2,SubTreeSelection=3,Tiered=4}}
        function realReset()
            reset(); AO.family='retail'; AO.PlanTalentImport=actualPlan; AO.TalentImportContentsMatch=actualMatch
            current=nativeOther; entries['8'].import_string=nativeOther
            op.expected.active_import_string=current; op.expected.entries=copy(entries)
            op.import_string=requested; op.allow_empty_hash=true
            C_Traits.GetTreeHash=function() local h={} for i=1,16 do h[i]=i end return h end
            C_Traits.GetTreeNodes=function() return {10} end
            C_Traits.GetNodeInfo=function() return {ID=10,type=0,maxRanks=2,entryIDs={101},activeEntry={entryID=101}} end
            C_Traits.GetEntryInfo=function() return {maxRanks=2} end
            frame.GetTreeInfo=function() return {ID=100} end
            frame.ReadLoadoutHeader=function(_,stream)
                local version,spec=stream:ExtractValue(8),stream:ExtractValue(16); local h={}
                for i=1,16 do h[i]=stream:ExtractValue(8) end
                return true,version,spec,h
            end
            frame.ReadLoadoutContent=function(_,stream)
                local selected=stream:ExtractValue(1)==1
                local purchased,partial,choice=false,false,false; local ranks,index=0,1
                if selected then
                    purchased=stream:ExtractValue(1)==1
                    if purchased then
                        partial=stream:ExtractValue(1)==1
                        if partial then ranks=stream:ExtractValue(6) end
                        choice=stream:ExtractValue(1)==1
                        if choice then index=stream:ExtractValue(2)+1 end
                    end
                end
                return {{isNodeSelected=selected,isNodeGranted=selected and not purchased,isPartiallyRanked=partial,
                    isChoiceNode=choice,partialRanksPurchased=ranks,choiceNodeSelection=index}}
            end
            frame.ConvertToImportLoadoutEntryInfo=function(_,active,tree,content)
                assert(active==7 and tree==100 and content[1].partialRanksPurchased==1)
                return {{nodeID=10,selectionEntryID=101,ranksGranted=0,ranksPurchased=1}}
            end
            C_ClassTalents.ImportLoadout=function(id,nodes,name,text)
                assert(id==7 and name=='New' and text==requested and #nodes==1)
                assert(nodes[1].nodeID==10 and nodes[1].selectionEntryID==101 and nodes[1].ranksPurchased==1 and nodes[1].ranksGranted==0)
                writes=writes+1; nodes[1].ranksPurchased=99
                if checkReceipt then assert(AzerothOperatorOperations==op.id..':outcome_unknown:import_talent_config;') end
                if mode=='throw_before' then error('before') end
                if mode~='delay' then realFinish() end
                if mode=='throw_after' then error('after') end
                return true
            end
        end
        function realFinish()
            current=mode=='wrong_active' and nativeOther or nativeDesired
            entries['11']={name='New',uses_shared_action_bars=false,import_string=mode=='wrong_saved' and nativeOther or nativeDesired,serialization_version=2}
            selected=11; frame.lastSelectedConfigID=11; uiSelection=11
            if mode=='changed_tree' then C_Traits.GetTreeNodes=function() return {20} end end
        end
        realReset(); assert(AO.ApplyTalentImport(op).status=='applied' and writes==1)
        assert(current~=op.expected.active_import_string and entries['8'].import_string==nativeOther)
        for _,failure in ipairs({'wrong_active','wrong_saved','changed_tree'}) do
            realReset(); mode=failure; assert(AO.ApplyTalentImport(op).status=='outcome_unknown' and writes==1,failure)
        end
        realReset(); mode='delay'; assert(AO.ApplyTalentImport(op).status=='outcome_unknown' and writes==1)
        realFinish(); assert(AO.VerifyTalentImport(op) and writes==1)
        realReset(); op.allow_empty_hash=false
        assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
        realReset(); frame.ConvertToImportLoadoutEntryInfo=function() return {} end
        assert(not pcall(AO.ApplyTalentImport,op) and writes==0)
    "#).exec()?;
    let snapshot: azeroth_operator::Snapshot = serde_json::from_value(serde_json::json!({
        "schema_version":1,"source":"fixture","captured_at":"2026-11-04T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},
        "character":{"name":"Fixture","realm":"Synthetic"},"sections":{"talents":{
        "config_id":7,"nodes":{},"loadout_export":{"status":"available","format":"blizzard_traits","import_string":import_text(false,2),"spec_id":263,"serialization_version":2,"staged_changes":false},
        "saved_loadouts":{"spec_id":263,"selected_config_id":8,"entries":{"8":{"name":"Old","uses_shared_action_bars":true,"import_string":import_text(false,2),"serialization_version":2}}}}}
    }))?;
    let request = serde_json::json!({"operation":"import","name":"New","import_string":import_text(true,1),"allow_empty_hash":true,"expected_new_shared_action_bars":false});
    let compile = |r: serde_json::Value| {
        azeroth_operator::talent_operation::compile(
            &snapshot,
            &"a".repeat(64),
            &serde_json::from_value(r)?,
        )
    };
    let prepared = compile(request.clone())?;
    assert_eq!(prepared["execution_supported"], false);
    assert_eq!(prepared["changes_active_talents"], true);
    let code = prepared["import_code"].as_str().unwrap();
    assert_eq!(code.split(':').count(), 26);
    lua.load(include_str!("../addon/TalentImportDecode.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeTalentImport")?;
    let decoded: mlua::Table = decoder.call(code)?;
    assert_eq!(
        decoded.get::<String>("import_string")?,
        import_text(true, 1)
    );
    lua.load("realReset()").exec()?;
    lua.globals().set("op", decoded)?;
    lua.load("assert(AO.ApplyTalentImport(op).status=='applied' and writes==1)")
        .exec()?;
    lua.globals().set("code", code)?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load(r#"
        function GetTime() return now end
        function setupImport()
            realReset(); AzerothOperatorOperations=''; now=0; checkReceipt=true
            op=AO.DecodeTalentImport(code)
        end
        setupImport(); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==0)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code..':-') and writes==0)
        AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
        assert(ok and r.status=='applied' and writes==1)
        AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        SlashCmdList={}; function GetServerTime() return 0 end
        function date() return '2026-11-04T00:00:00Z' end
        AO.collectors={operation_receipts=function() return AO.ReadMacroReceipts() end}
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("AO.Capture()").exec()?;
    let receipt: String = lua.globals().get("AzerothOperatorOperations")?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let imported = azeroth_operator::import::decode(
        format!("AzerothOperatorExport = \"{export}\"\nAzerothOperatorOperations = \"{receipt}\"")
            .as_bytes(),
    )?;
    assert_eq!(
        imported.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()],
        serde_json::json!({"kind":"import_talent_config","status":"applied"})
    );
    lua.load(r#"
        setupImport(); mode='delay'; AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
        assert(ok and r.status=='outcome_unknown' and writes==1)
        realFinish(); assert(AO.ReadMacroReceipts()[op.id].status=='applied' and writes==1)
        setupImport(); mode='delay'; AO.ReviewMacroImport(code); AO.ApplyReviewedMacro(code)
        now=61; realFinish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown' and writes==1)
        for _,failure in ipairs({'throw_before','throw_after','callback_error'}) do
            setupImport(); mode=failure; AO.ReviewMacroImport(code); local r,ok=AO.ApplyReviewedMacro(code)
            assert(not ok and r.status=='outcome_unknown' and writes==1,failure)
            assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown')
            AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
        end
        setupImport(); mode='delay'; AO.ReviewMacroImport(code); AO.ApplyReviewedMacro(code)
    "#).exec()?;
    lua.load(include_str!("../addon/MacroImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    lua.load("realFinish(); assert(AO.ReadMacroReceipts()[op.id].status=='outcome_unknown'); AO.ReviewMacroImport(code); assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)").exec()?;
    for field in [
        "import_string",
        "allow_empty_hash",
        "expected_new_shared_action_bars",
        "name",
    ] {
        let mut invalid = request.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(compile(invalid).is_err());
    }
    for (field, value) in [
        ("allow_empty_hash", serde_json::json!(false)),
        ("import_string", serde_json::json!("AAAA")),
        ("target_config_id", serde_json::json!(9)),
        ("operation", serde_json::json!("create")),
    ] {
        let mut invalid = request.clone();
        invalid[field] = value;
        assert!(compile(invalid).is_err());
    }
    let mut full = request.clone();
    full["import_string"] = serde_json::json!(import_text(false, 1));
    full["allow_empty_hash"] = serde_json::json!(false);
    assert_ne!(compile(full)?["operation_id"], prepared["operation_id"]);
    for (index, value) in [
        (0, "AO-TALCREATE-1"),
        (10, "9"),
        (17, "128"),
        (19, "2"),
        (20, "4141"),
        (21, "7"),
    ] {
        let mut fields: Vec<_> = code.split(':').collect();
        fields[index] = value;
        assert!(decoder.call::<mlua::Table>(fields.join(":")).is_err());
    }
    assert!(
        decoder
            .call::<mlua::Table>(format!("{code}:extra"))
            .is_err()
    );
    Ok(())
}

fn import_text(empty_hash: bool, ranks: u32) -> String {
    let mut bits = Vec::new();
    let mut add = |width, value: u32| {
        for bit in 0..width {
            bits.push((value >> bit) & 1);
        }
    };
    add(8, 2);
    add(16, 263);
    for byte in 1..=16 {
        add(8, if empty_hash { 0 } else { byte });
    }
    add(1, 1);
    add(1, 1);
    add(1, u32::from(ranks < 2));
    if ranks < 2 {
        add(6, ranks);
    }
    add(1, 0);
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    bits.chunks(6)
        .map(|chunk| {
            alphabet[chunk
                .iter()
                .enumerate()
                .map(|(i, b)| (*b as usize) << i)
                .sum::<usize>()] as char
        })
        .collect()
}
