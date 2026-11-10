use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn local_filtered_recipe_view_preserves_unknowns_and_rejects_external_contexts()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={};mode='normal';reads=0;infoReads={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return value=='PRIVATE-SOURCE' or (mode=='secret_id' and value==7) or (mode=='secret_name' and value=='Recipe') end
        ProfessionsFrame={CraftingPage={IsVisible=function() return mode~='hidden' end,SchematicForm={}}}
        Professions={InLocalCraftingMode=function() return mode~='nonlocal' end,
            IsViewingExternalCraftingList=function() return mode=='linked' or mode=='guild' end,
            GetProfessionInfo=function()
                local rank=mode=='rank_secret' and 'PRIVATE-SOURCE' or (mode=='rank_changed' and reads>1 and 101 or 100)
                if mode=='rank_missing' then rank=nil end
                local modifier=mode=='modifier_secret' and 'PRIVATE-SOURCE'
                    or (mode=='modifier_changed' and reads>1 and -6 or -5)
                if mode=='modifier_zero' then modifier=0 end
                return {professionID=mode=='profession_changed' and reads>1 and 165 or 164,
                    skillLevel=rank,maxSkillLevel=mode=='rank_invalid' and 99 or 150,skillModifier=modifier}
            end}
        C_TradeSkillUI={IsNPCCrafting=function() return mode=='npc' end,
            IsDataSourceChanging=function()
                if mode=='secret_source' then return 'PRIVATE-SOURCE' end
                if mode=='unknown_source' then return nil end
                return mode=='rebuilding' or (mode=='rebuilding_end' and reads>1)
            end,
            GetFilteredRecipeIDs=function()
                reads=reads+1
                if mode=='empty' then return {} end
                if mode=='oversize' then local t={};for i=1,4097 do t[i]=i end;return t end
                if mode=='duplicate' then return {7,7} end
                if mode=='sparse' then return {[1]=7,[3]=9} end
                if mode=='changed' and reads>1 then return {7,8,10} end
                return {7,8,9}
            end,
            GetRecipeInfo=function(id)
                infoReads[id]=(infoReads[id] or 0)+1
                if id==9 then return nil end
                if mode=='metadata_error' and infoReads[id]>1 then error('PRIVATE-SOURCE') end
                return {recipeID=mode=='wrong_id' and 99 or id,name='Recipe',learned=id==7,
                    relativeDifficulty=mode=='metadata_secret' and 'PRIVATE-SOURCE' or mode=='metadata_invalid' and 4 or (mode=='metadata_changed' and infoReads[id]>1) and 1 or id==7 and 0 or 3,
                    canSkillUp=mode=='metadata_secret' and 'PRIVATE-SOURCE' or mode=='metadata_invalid' and 1 or (mode=='metadata_changed' and infoReads[id]>1) or id==7,
                    numSkillUps=mode=='metadata_secret' and 'PRIVATE-SOURCE' or mode=='metadata_invalid' and -1 or (mode=='metadata_changed' and infoReads[id]>1) and 3 or id==7 and 2 or 0,
                    crafterName='DO-NOT-EXPORT',playerGUID='DO-NOT-EXPORT'}
            end,
            CraftRecipe=function() error('craft forbidden') end,
            SetShowLearned=function() error('filter mutation forbidden') end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("reads=0;infoReads={};AO.Capture()").exec()?;
        let encoded: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{encoded}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["profession_recipe_view"]["profession_id"],
        164
    );
    assert_eq!(
        saved.sections["profession_recipe_view"]["rank_observation"],
        serde_json::json!({"rank":100,"max_rank":150,"skill_modifier":-5})
    );
    for mode in ["modifier_secret", "modifier_changed", "modifier_zero"] {
        lua.globals().set("mode", mode)?;
        let observed = capture()?;
        let rank = &observed.sections["profession_recipe_view"]["rank_observation"];
        assert_eq!(rank["rank"], 100, "{mode}");
        if mode == "modifier_zero" {
            assert_eq!(rank["skill_modifier"], 0);
        } else {
            assert!(rank.get("skill_modifier").is_none(), "{mode}");
        }
    }
    for mode in [
        "rank_secret",
        "rank_missing",
        "rank_invalid",
        "rank_changed",
    ] {
        lua.globals().set("mode", mode)?;
        let observed = capture()?;
        let view = &observed.sections["profession_recipe_view"];
        assert_eq!(view["profession_id"], 164, "{mode}");
        assert!(view.get("rank_observation").is_none(), "{mode}");
    }
    assert_eq!(
        saved.sections["profession_recipe_view"]["entries"]["9"],
        serde_json::json!({})
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["profession_recipe_view_counts"],
        serde_json::json!({"observed":3,"learned":1,"unlearned":1,"learning_unknown":1})
    );
    assert!(!serde_json::to_string(&saved)?.contains("DO-NOT-EXPORT"));
    assert_eq!(
        saved.sections["profession_recipe_view"]["entries"]["7"]["relative_difficulty"],
        "optimal"
    );
    assert_eq!(
        saved.sections["profession_recipe_view"]["entries"]["7"]["skill_ups_reported"],
        2
    );
    assert_eq!(
        saved.sections["profession_recipe_view"]["entries"]["8"]["can_skill_up"],
        false
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["profession_skill_up_candidates"]["candidate_count"],
        1
    );
    for mode in ["metadata_secret", "metadata_invalid", "metadata_error"] {
        lua.globals().set("mode", mode)?;
        let observed = capture()?;
        let row = &observed.sections["profession_recipe_view"]["entries"]["7"];
        for field in ["relative_difficulty", "can_skill_up", "skill_ups_reported"] {
            assert!(row.get(field).is_none(), "{mode}/{field}");
        }
        assert_eq!(row["learned"], true);
        assert!(!serde_json::to_string(&observed)?.contains("PRIVATE-SOURCE"));
    }
    lua.globals().set("mode", "metadata_changed")?;
    let observed = capture()?;
    let row = &observed.sections["profession_recipe_view"]["entries"]["7"];
    assert!(row.get("relative_difficulty").is_none());
    assert!(row.get("skill_ups_reported").is_none());
    assert_eq!(row["can_skill_up"], true);
    lua.load("mode='empty'").exec()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&capture()?)["profession_recipe_view_counts"]["observed"],
        0
    );
    for mode in [
        "hidden",
        "nonlocal",
        "linked",
        "guild",
        "npc",
        "secret_id",
        "secret_name",
        "duplicate",
        "sparse",
        "oversize",
        "wrong_id",
        "changed",
        "profession_changed",
        "rebuilding",
        "rebuilding_end",
        "unknown_source",
        "secret_source",
    ] {
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?
                .unavailable
                .contains_key("profession_recipe_view"),
            "{mode}"
        );
    }
    lua.load("mode='normal';ProfessionsFrame.CraftingPage.SchematicForm.isInspection=true")
        .exec()?;
    assert!(
        capture()?
            .unavailable
            .contains_key("profession_recipe_view")
    );
    lua.load("ProfessionsFrame.CraftingPage.SchematicForm.isInspection=false;C_TradeSkillUI.IsDataSourceChanging=nil").exec()?;
    assert!(
        capture()?
            .unavailable
            .contains_key("profession_recipe_view")
    );
    Ok(())
}
