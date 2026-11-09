use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn bulk_materials_preserve_partial_failures_and_local_context()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0;schematics=0
        craftableReads=0;craftableByID={};cooldownReads=0;cooldownByID={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={TradeskillRecipeType={Item=1,Enchant=2}}
        ProfessionsFrame={CraftingPage={IsVisible=function() return mode~='hidden' end,SchematicForm={}}}
        Professions={InLocalCraftingMode=function() return true end,
            IsViewingExternalCraftingList=function() return false end,
            GetProfessionInfo=function() return {professionID=mode=='changed_context' and schematics>0 and 165 or 164} end}
        C_TradeSkillUI={IsNPCCrafting=function() return false end,IsDataSourceChanging=function() return false end,
            GetProfessionSpells=function()
                reads=reads+1
                if mode=='empty' then return {} end
                if mode=='budget' then local t={};for i=1,257 do t[i]=i end;return t end
                if mode=='changed_ids' and schematics>0 then return {2675} end
                return {2675,3508}
            end,
            GetRecipeInfo=function(id) return {recipeID=id,learned=mode~='unlearned'} end,
            GetRecipeSchematic=function(id,recraft)
                assert(recraft==false);schematics=schematics+1
                if id==3508 and mode~='budget' then error('PRIVATE') end
                local reagent={required=true,reagents={{itemID=2840}},variableQuantities={},quantityRequired=3}
                local slots={reagent,{required=true,reagents={{itemID=2840}},variableQuantities={},quantityRequired=2}}
                if mode=='unsupported' then reagent.reagents={{itemID=2840},{itemID=2841}} end
                if mode=='variable' then reagent.variableQuantities={1} end
                if mode=='currency' then reagent.reagents={{currencyID=1}} end
                if mode=='secret' then reagent.quantityRequired='PRIVATE' end
                if mode=='changed_quantity' and schematics%2==0 then reagent.quantityRequired=4 end
                if mode=='sparse' then slots={[1]=reagent,[3]=reagent} end
                if mode=='empty_materials' then slots={} end
                if mode=='budget' then slots={};for i=1,64 do slots[i]={required=true,reagents={{itemID=i}},variableQuantities={},quantityRequired=1} end end
                local result={recipeID=id,isRecraft=false,recipeType=1,outputItemID=2870,quantityMin=1,quantityMax=2,reagentSlotSchematics=slots}
                if mode=='unknown_kind' then result.recipeType=nil;Enum.TradeskillRecipeType.Enchant=nil end
                return result
            end,
            GetCraftableCount=function(id)
                craftableReads=craftableReads+1;craftableByID[id]=(craftableByID[id] or 0)+1
                if mode=='count_value' then return 7 end
                if mode=='count_zero' then return 0 end
                if mode=='count_max' then return 4294967295 end
                if mode=='count_negative' then return -1 end
                if mode=='count_fraction' then return 1.5 end
                if mode=='count_large' then return 4294967296 end
                if mode=='count_secret' then return 'PRIVATE' end
                if mode=='count_bool' then return false end
                if mode=='count_infinite' then return math.huge end
                if mode=='count_nan' then return 0/0 end
                if mode=='count_error' then error('PRIVATE') end
                if mode=='count_changed' then return craftableReads end
            end,
            GetRecipeCooldown=function(id)
                cooldownReads=cooldownReads+1;cooldownByID[id]=(cooldownByID[id] or 0)+1
                if mode=='cooldown_value' then return 12.1,false,1,3 end
                if mode=='cooldown_zero' then return 0,false,0,3 end
                if mode=='cooldown_charges' then return nil,false,2,3 end
            end,
            CraftRecipe=function() error('no crafting') end,
            SetShowLearned=function() error('no filters') end,
            SetShowUnlearned=function() error('no filters') end}
        countImpl=C_TradeSkillUI.GetCraftableCount
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads=0;schematics=0;craftableReads=0;craftableByID={};cooldownReads=0;cooldownByID={};C_TradeSkillUI.GetCraftableCount=countImpl; if mode=='count_missing' then C_TradeSkillUI.GetCraftableCount=nil end;Enum.TradeskillRecipeType.Enchant=2;AO.Capture()")
            .exec()?;
        let value: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{value}\"").as_bytes(),
        )?)
    };
    let normal = capture("normal")?;
    assert_eq!(
        normal.sections["profession_recipe_materials"]["entries"],
        serde_json::json!({
        "2675":{"status":"fixed_items","reagents":{"2840":5},"output_item_id":2870,"base_output_quantity":{"minimum":1,"maximum":2}},
        "3508":{"status":"unavailable"}})
    );
    assert!(!serde_json::to_string(&normal)?.contains("PRIVATE"));
    for (mode, expected) in [
        (
            "cooldown_value",
            serde_json::json!({"remaining_seconds":13,"daily":false,"charges":1,"max_charges":3}),
        ),
        (
            "cooldown_zero",
            serde_json::json!({"remaining_seconds":0,"daily":false,"charges":0,"max_charges":3}),
        ),
        (
            "cooldown_charges",
            serde_json::json!({"charges":2,"max_charges":3}),
        ),
    ] {
        let captured = capture(mode)?;
        assert_eq!(
            captured.sections["profession_recipe_materials"]["entries"]["2675"]["cooldown"],
            expected
        );
        assert_eq!(lua.globals().get::<u32>("cooldownReads")?, 2);
        let by_id: mlua::Table = lua.globals().get("cooldownByID")?;
        assert_eq!(by_id.get::<Option<u32>>(3508)?, None);
    }
    for (mode, expected) in [
        ("count_value", 7u64),
        ("count_zero", 0),
        ("count_max", u32::MAX as u64),
    ] {
        let captured = capture(mode)?;
        assert_eq!(
            captured.sections["profession_recipe_materials"]["entries"]["2675"]["reported_craftable_count"],
            expected
        );
        assert_eq!(lua.globals().get::<u32>("craftableReads")?, 2);
        let by_id: mlua::Table = lua.globals().get("craftableByID")?;
        assert_eq!(by_id.get::<Option<u32>>(3508)?, None);
    }
    for mode in [
        "count_missing",
        "count_negative",
        "count_fraction",
        "count_large",
        "count_secret",
        "count_bool",
        "count_infinite",
        "count_nan",
        "count_error",
        "count_changed",
    ] {
        let captured = capture(mode)?;
        assert_eq!(
            captured.sections["profession_recipe_materials"]["entries"]["2675"],
            normal.sections["profession_recipe_materials"]["entries"]["2675"],
            "{mode}"
        );
        assert!(!serde_json::to_string(&captured)?.contains("PRIVATE"));
    }
    for mode in ["unsupported", "variable", "currency"] {
        assert_eq!(
            capture(mode)?.sections["profession_recipe_materials"]["entries"]["2675"],
            serde_json::json!({"status":"unsupported_required_slots"})
        );
        assert_eq!(lua.globals().get::<u32>("cooldownReads")?, 0);
    }
    for mode in [
        "secret",
        "changed_quantity",
        "sparse",
        "unlearned",
        "unknown_kind",
    ] {
        let value = capture(mode)?;
        assert_eq!(
            value.sections["profession_recipe_materials"]["entries"]["2675"],
            serde_json::json!({"status":"unavailable"}),
            "{mode}"
        );
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    for mode in ["hidden", "changed_context", "changed_ids"] {
        assert!(
            !capture(mode)?
                .sections
                .contains_key("profession_recipe_materials"),
            "{mode}"
        );
    }
    assert_eq!(
        capture("empty")?.sections["profession_recipe_materials"]["entries"],
        serde_json::json!({})
    );
    assert_eq!(
        capture("empty_materials")?.sections["profession_recipe_materials"]["entries"]["2675"]["reagents"],
        serde_json::json!({})
    );
    let budget = capture("budget")?;
    let entries = budget.sections["profession_recipe_materials"]["entries"]
        .as_object()
        .unwrap();
    assert_eq!(entries.len(), 257);
    assert_eq!(entries["256"]["status"], "fixed_items");
    assert_eq!(entries["257"]["status"], "budget_exhausted");
    let by_id: mlua::Table = lua.globals().get("craftableByID")?;
    assert_eq!(by_id.get::<Option<u32>>(257)?, None);
    let cooldown_by_id: mlua::Table = lua.globals().get("cooldownByID")?;
    assert_eq!(cooldown_by_id.get::<Option<u32>>(257)?, None);
    assert_eq!(
        azeroth_operator::analysis::summary(&budget)["profession_recipe_material_observations"]["fixed_item_recipe_count"],
        256
    );
    Ok(())
}
