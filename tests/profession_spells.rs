use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn profession_spells_require_stable_public_local_context() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' or (mode=='secret_id' and v==2675) end
        ProfessionsFrame={CraftingPage={IsVisible=function() return mode~='hidden' end,SchematicForm={}}}
        Professions={InLocalCraftingMode=function() return mode~='remote' end,
            IsViewingExternalCraftingList=function() return mode=='external' end,
            GetProfessionInfo=function()
                if mode=='secret_profession' then return {professionID='PRIVATE'} end
                return {professionID=mode=='changed_profession' and reads>1 and 165 or 164}
            end}
        C_TradeSkillUI={IsNPCCrafting=function() return mode=='npc' end,
            IsDataSourceChanging=function()
                if mode=='unknown_context' then return nil end
                if mode=='secret_context' then return 'PRIVATE' end
                return mode=='changing' or (mode=='changing_end' and reads>1)
            end,
            GetProfessionSpells=function(profession)
                assert(profession==164);reads=reads+1
                if mode=='empty' then return {} end
                if mode=='error' or (mode=='second_error' and reads>1) then error('PRIVATE') end
                if mode=='secret_table' then return 'PRIVATE' end
                if mode=='duplicate' then return {2675,2675} end
                if mode=='sparse' then return {[1]=2675,[3]=3508} end
                if mode=='map' then return {x=2675} end
                if mode=='bad_id' then return {0} end
                if mode=='float_id' then return {1.5} end
                if mode=='large_id' then return {4294967296} end
                if mode=='oversize' then local t={};for i=1,4097 do t[i]=i end;return t end
                if mode=='changed' and reads>1 then return {2675,3511} end
                if mode=='reordered' and reads>1 then return {3508,2675} end
                return {2675,3508}
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
        lua.load("reads=0;AO.Capture()").exec()?;
        let value: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{value}\"").as_bytes(),
        )?)
    };
    let normal = capture("normal")?;
    assert_eq!(
        normal.sections["profession_spells"],
        serde_json::json!({
        "coverage":"reported_known_profession_spells","profession_id":164,"entries":{"2675":{},"3508":{}}})
    );
    assert_eq!(
        capture("reordered")?.sections["profession_spells"],
        normal.sections["profession_spells"]
    );
    assert_eq!(
        azeroth_operator::analysis::summary(&capture("empty")?)["profession_spell_observations"]["reported_spell_count"],
        0
    );
    for mode in [
        "hidden",
        "remote",
        "external",
        "npc",
        "changing",
        "changing_end",
        "unknown_context",
        "secret_context",
        "secret_profession",
        "changed_profession",
        "error",
        "second_error",
        "secret_table",
        "secret_id",
        "duplicate",
        "sparse",
        "map",
        "bad_id",
        "float_id",
        "large_id",
        "oversize",
        "changed",
    ] {
        let value = capture(mode)?;
        assert!(!value.sections.contains_key("profession_spells"), "{mode}");
        assert!(
            value.unavailable.contains_key("profession_spells"),
            "{mode}"
        );
        assert_eq!(value.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&value)?.contains("PRIVATE"));
    }
    lua.load("ProfessionsFrame.CraftingPage.SchematicForm.isInspection=true")
        .exec()?;
    assert!(
        !capture("normal")?
            .sections
            .contains_key("profession_spells")
    );
    lua.load("ProfessionsFrame.CraftingPage.SchematicForm.isInspection=false;C_TradeSkillUI.GetProfessionSpells=nil").exec()?;
    assert!(
        !capture("normal")?
            .sections
            .contains_key("profession_spells")
    );
    Ok(())
}
