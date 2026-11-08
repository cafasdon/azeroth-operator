use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn bulk_materials_preserve_partial_failures_and_local_context()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0;schematics=0
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
            CraftRecipe=function() error('no crafting') end,
            SetShowLearned=function() error('no filters') end,
            SetShowUnlearned=function() error('no filters') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("reads=0;schematics=0;Enum.TradeskillRecipeType.Enchant=2;AO.Capture()")
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
    for mode in ["unsupported", "variable", "currency"] {
        assert_eq!(
            capture(mode)?.sections["profession_recipe_materials"]["entries"]["2675"],
            serde_json::json!({"status":"unsupported_required_slots"})
        );
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
    assert_eq!(
        azeroth_operator::analysis::summary(&budget)["profession_recipe_material_observations"]["fixed_item_recipe_count"],
        256
    );
    Ok(())
}
