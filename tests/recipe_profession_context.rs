use mlua::Lua;

#[test]
fn selected_recipe_omits_unmatched_or_restricted_profession_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let lua=Lua::new();
    lua.load(r#"
        AO={family='forever'}; current=165; recipeProfession=165
        function issecretvalue(v) return v=='PRIVATE' end
        Professions={InLocalCraftingMode=function() return true end,
            GetProfessionInfo=function() return {professionID=current} end}
        C_TradeSkillUI={IsNPCCrafting=function() return false end,
            GetProfessionInfoByRecipeID=function() return {professionID=recipeProfession,
                professionName='Leatherworking',skillLevel=50,maxSkillLevel=75} end}
        Enum={TradeskillRecipeType={Item=1,Enchant=2}}
        ProfessionsFrame={CraftingPage={SchematicForm={IsVisible=function() return true end,
            GetRecipeInfo=function() return {recipeID=1229432,name='Camp Tent',learned=true} end,
            recipeSchematic={recipeID=1229432,isRecraft=false,recipeType=1,outputItemID=279978,
                reagentSlotSchematics={{required=true,reagents={{itemID=2318}},
                    quantityRequired=5,variableQuantities={}}}}}}}
    "#).exec()?;
    let ao:mlua::Table=lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua")).call::<()>(("AzerothOperator",ao))?;
    let capture=||lua.load("return AO.collectors.selected_recipe()").eval::<mlua::Table>();
    assert!(capture()?.get::<mlua::Table>("profession").is_ok());
    for (context,recipe) in [("165","2945"),("'PRIVATE'","165"),("165","'PRIVATE'"),("nil","165")] {
        lua.load(format!("current={context};recipeProfession={recipe}")).exec()?;
        let result=capture()?;
        assert!(result.get::<Option<mlua::Table>>("profession")?.is_none());
        assert_eq!(result.get::<u32>("recipe_id")?,1229432);
        assert_eq!(result.get::<mlua::Table>("reagents")?.get::<u32>("2318")?,5);
    }
    Ok(())
}
