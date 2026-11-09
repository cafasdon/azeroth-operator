use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn optional_provider_exports_only_bounded_stable_public_references()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(
        r#"
        AO={family='forever',variant='live'};SlashCmdList={}; mode='normal';gets=0;seen={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE' end
        function setup()
            gets=0;seen={}
            LibQuestieDB={RequireContract=function(v) assert(v==1);return mode~='contract' end,
                ModeIndicator={GetStatus=function() return {
                    expansion=mode=='flavour' and 'Vanilla' or 'Forever',
                    mode=mode=='mode' and 'PRIVATE' or 'baked',
                    contractVersion=mode=='context_change' and gets>0 and 5 or 4} end},
                Item={Exists=function(id)
                    if mode=='exists_error' then error('PRIVATE') end
                    if mode=='exists_secret' then return 'PRIVATE' end
                    return mode~='unknown'
                end,Get=function(id,key)
                    gets=gets+1;seen[id]=true
                    if key=='vendors' then return nil end
                    if key~='npcDrops' then return {} end
                    if mode=='secret' then return {'PRIVATE'} end
                    if mode=='error' then error('PRIVATE') end
                    if mode=='sparse' then return {[2]=1} end
                    if mode=='large' then local t={};for i=1,4097 do t[i]=i end;return t end
                    if mode=='changing' then return {gets} end
                    if mode=='truncated' then local t={};for i=10,1,-1 do t[#t+1]=i end;return t end
                    return {9,2,1,2}
                end}}
            if mode=='missing' then LibQuestieDB=nil end
        end
    "#,
    )
    .exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(r#"
        AO.collectors.profession_recipe_materials=function()
            if mode=='missing_materials' then return end
            local first={};local second={}
            if mode=='limit' then
                for i=1,64 do first[tostring(i)]=1 end;second['65']=1
            elseif mode~='empty' then first['2840']=2;first['3470']=1 end
            return {coverage='reported_known_spell_schematics',profession_id=164,entries={
                ['101']={status='fixed_items',reagents=first},['102']={status='fixed_items',reagents=second}}}
        end
    "#).exec()?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("setup(); AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport=\"{text}\"").as_bytes(),
        )?)
    };
    let normal = capture("normal")?;
    let section = &normal.sections["recipe_item_sources"];
    assert_eq!(section["reported_item_count"], 2);
    assert_eq!(section["provider_contract_version"], 4);
    assert_eq!(
        section["entries"]["2840"]["npc_drops"],
        serde_json::json!({
        "entries":{"1":true,"2":true,"9":true},"distinct_id_count":3,"truncated":false})
    );
    assert_eq!(
        section["entries"]["2840"]["object_drops"]["distinct_id_count"],
        0
    );
    assert!(section["entries"]["2840"].get("vendors").is_none());
    for mode in ["secret", "error", "sparse", "large"] {
        let value = capture(mode)?;
        let row = &value.sections["recipe_item_sources"]["entries"]["2840"];
        assert_eq!(row["status"], "reported");
        assert!(row.get("npc_drops").is_none(), "{mode}");
        assert!(row.get("object_drops").is_some());
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    for mode in ["changing", "exists_error", "exists_secret"] {
        assert_eq!(
            capture(mode)?.sections["recipe_item_sources"]["entries"]["2840"],
            serde_json::json!({"status":"unavailable"})
        );
    }
    assert_eq!(
        capture("unknown")?.sections["recipe_item_sources"]["entries"]["2840"],
        serde_json::json!({"status":"not_in_provider"})
    );
    let truncated = capture("truncated")?;
    let row = &truncated.sections["recipe_item_sources"]["entries"]["2840"]["npc_drops"];
    assert_eq!(row["distinct_id_count"], 10);
    assert_eq!(row["truncated"], true);
    assert_eq!(row["entries"].as_object().unwrap().len(), 8);
    assert!(row["entries"].get("9").is_none());
    for mode in [
        "missing",
        "contract",
        "flavour",
        "mode",
        "context_change",
        "missing_materials",
    ] {
        assert!(
            !capture(mode)?.sections.contains_key("recipe_item_sources"),
            "{mode}"
        );
    }
    assert_eq!(
        capture("empty")?.sections["recipe_item_sources"]["entries"],
        serde_json::json!({})
    );
    let limit = capture("limit")?;
    assert_eq!(
        limit.sections["recipe_item_sources"]["reported_item_count"],
        65
    );
    assert_eq!(
        limit.sections["recipe_item_sources"]["entries"]
            .as_object()
            .unwrap()
            .len(),
        64
    );
    let seen: mlua::Table = lua.globals().get("seen")?;
    assert_eq!(seen.get::<Option<bool>>(65)?, None);
    Ok(())
}
