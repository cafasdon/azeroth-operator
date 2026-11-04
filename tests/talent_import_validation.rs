use mlua::Lua;

fn encoded(version: u32, spec: u32, empty: bool, selected: bool) -> String {
    let mut bits = Vec::new();
    let mut add = |width, value: u32| {
        for bit in 0..width {
            bits.push((value >> bit) & 1);
        }
    };
    add(8, version);
    add(16, spec);
    for byte in 1..=16 {
        add(8, if empty { 0 } else { byte });
    }
    add(1, u32::from(selected));
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

#[test]
fn native_parser_preflight_is_bounded_and_read_only() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail'}; reads=0
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
        C_ClassTalents={GetActiveConfigID=function() return 7 end}
        C_SpecializationInfo={GetSpecialization=function() return 1 end,GetSpecializationInfo=function() return 263 end}
        C_Traits={ConfigHasStagedChanges=function() return staged or false end,GetLoadoutSerializationVersion=function() return 2 end,
            GetTreeHash=function() local h={} for i=1,16 do h[i]=i end; if mode=='hash' then h[1]=99 end; return h end,
            GetTreeNodes=function()
                reads=reads+1
                if mode=='sparse' then return {[1]=10,[3]=20} end
                if mode=='duplicate' then return {10,10} end
                return {mode=='race' and reads>1 and 20 or 10}
            end}
        frame={IsShown=function() return true end,IsInspecting=function() return false end,IsCommitInProgress=function() return false end,
            GetConfigID=function() return 7 end,GetTreeInfo=function() return {ID=100} end,
            ReadLoadoutHeader=function(_,stream)
                local version,spec=stream:ExtractValue(8),stream:ExtractValue(16); local h={}
                for i=1,16 do h[i]=stream:ExtractValue(8) end; return true,version,spec,h
            end,
            ReadLoadoutContent=function(_,stream)
                local selected=stream:ExtractValue(1)==1
                if selected then stream:ExtractValue(6) end
                if mode=='missing' then return {} end
                return {{isNodeSelected=selected,isNodeGranted=false,isPartiallyRanked=mode=='rank',isChoiceNode=false,partialRanksPurchased=0,choiceNodeSelection=1}}
            end}
        PlayerSpellsFrame={TalentsFrame=frame}
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/TalentImportValidation.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let inspect: mlua::Function = ao.get("InspectTalentImport")?;
    let text = encoded(2, 263, false, false);
    let result: mlua::Table = inspect.call(text.clone())?;
    assert!(!result.get::<bool>("game_execution")?);
    assert!(!result.get::<bool>("semantic_legality_verified")?);
    assert!(!result.get::<bool>("hash_omitted")?);
    let observation: mlua::Table = result.get("observation")?;
    assert_eq!(observation.get::<u32>("tree_id")?, 100);
    for bad in [
        encoded(3, 263, false, false),
        encoded(2, 264, false, false),
        encoded(2, 263, false, true),
        encoded(2, 263, true, false),
        format!("{text}A"),
        format!("{text}="),
        text[..25].to_owned(),
        "A".repeat(16385),
    ] {
        assert!(inspect.call::<mlua::Table>(bad).is_err());
    }
    let omitted: mlua::Table = inspect.call((encoded(2, 263, true, false), true))?;
    assert!(omitted.get::<bool>("hash_omitted")?);
    let matches: mlua::Function = ao.get("TalentImportContentsMatch")?;
    assert!(matches.call::<bool>((omitted.clone(), text.clone()))?);
    assert!(matches.call::<bool>((result.clone(), text.clone()))?);
    assert!(
        matches
            .call::<bool>((result.clone(), encoded(2, 263, true, false)))
            .is_err()
    );
    lua.globals().set("expected", omitted)?;
    lua.globals().set("nativeText", text.clone())?;
    lua.load(r#"
        local function copy(t) local r={} for k,v in pairs(t) do r[k]=type(v)=='table' and copy(v) or v end return r end
        for _,field in ipairs({'isNodeSelected','isNodeGranted','isPartiallyRanked','isChoiceNode'}) do
            local changed=copy(expected); changed.content[1][field]=not changed.content[1][field]
            assert(not AO.TalentImportContentsMatch(changed,nativeText),field)
        end
        for _,field in ipairs({'partialRanksPurchased','choiceNodeSelection'}) do
            local changed=copy(expected); changed.content[1][field]=changed.content[1][field]+1
            assert(not AO.TalentImportContentsMatch(changed,nativeText),field)
        end
        for _,field in ipairs({'active_config_id','spec_id','tree_id','serialization_version'}) do
            local changed=copy(expected); changed.observation[field]=changed.observation[field]+1
            assert(not AO.TalentImportContentsMatch(changed,nativeText),field)
        end
        local changed=copy(expected); changed.observation.nodes[1]=20
        assert(not AO.TalentImportContentsMatch(changed,nativeText))
        changed=copy(expected); changed.observation.tree_hash[1]=99
        assert(not AO.TalentImportContentsMatch(changed,nativeText))
        local original=C_Traits.GetTreeNodes; local n=0
        C_Traits.GetTreeNodes=function() n=n+1; return {n<=2 and 10 or 20} end
        assert(not AO.TalentImportContentsMatch(expected,nativeText))
        C_Traits.GetTreeNodes=original
    "#).exec()?;
    let mut padding = text.clone();
    padding.pop();
    padding.push('/');
    assert!(inspect.call::<mlua::Table>(padding).is_err());
    lua.globals().set("text", text)?;
    lua.load(
        r#"
        for _,failure in ipairs({'hash','sparse','duplicate','race','missing','rank'}) do
            mode=failure; reads=0; assert(not pcall(AO.InspectTalentImport,text),failure)
        end
        mode=nil; staged=true; assert(not pcall(AO.InspectTalentImport,text)); staged=false
        function issecretvalue(v) return v==263 end
        assert(not pcall(AO.InspectTalentImport,text)); issecretvalue=nil
        AO.family='classic'; assert(not pcall(AO.InspectTalentImport,text))
    "#,
    )
    .exec()
}
