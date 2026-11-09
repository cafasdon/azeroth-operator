use azeroth_operator::import::decode;
use mlua::Lua;

fn fixture() -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}
        accesses=0; actions=0
        SECRET=setmetatable({}, {__index=function() accesses=accesses+1;error('PRIVATE access') end,
            __call=function() accesses=accesses+1;error('PRIVATE call') end,
            __eq=function() accesses=accesses+1;error('PRIVATE compare') end})
        function issecretvalue(v) return rawequal(v,SECRET) end
        function forbidden() actions=actions+1;error('PRIVATE action') end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() end
        CastSpellByID=forbidden;CastSpellByName=forbidden;PickupSpellBookItem=forbidden
        function reset()
            accesses=0;actions=0
            Enum={SpellBookSpellBank={Player=0},SpellBookItemType={Spell=1}}
            C_SpellBook={GetNumSpellBookSkillLines=function() return 1 end,
                GetSpellBookSkillLineInfo=function() return {shouldHide=false,isGuild=false,itemIndexOffset=0,numSpellBookItems=1} end,
                GetSpellBookItemInfo=function() return {itemType=1,actionID=100,spellID=101} end}
            C_Spell={GetSpellLevelLearned=function() return 20 end,
                IsSpellUsable=function() return true,false end,
                GetSpellCharges=function() return {currentCharges=1,maxCharges=2} end,
                GetSpellCastCount=function() return 3 end,
                GetSpellCooldown=function() return {isEnabled=true,isActive=false} end}
        end
        reset()
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

#[test]
fn required_surfaces_reject_without_touching_restricted_values()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    for setup in [
        "Enum=SECRET",
        "Enum=nil",
        "Enum=42",
        "Enum.SpellBookSpellBank=SECRET",
        "Enum.SpellBookItemType=SECRET",
        "Enum.SpellBookSpellBank.Player=SECRET",
        "Enum.SpellBookItemType.Spell=SECRET",
        "C_SpellBook=SECRET",
        "C_SpellBook=nil",
        "C_SpellBook=42",
        "C_SpellBook.GetNumSpellBookSkillLines=SECRET",
        "C_SpellBook.GetSpellBookSkillLineInfo=SECRET",
        "C_SpellBook.GetSpellBookItemInfo=SECRET",
        "C_SpellBook.GetNumSpellBookSkillLines=nil",
        "C_SpellBook.GetSpellBookSkillLineInfo=42",
        "C_SpellBook.GetSpellBookItemInfo=nil",
        "C_SpellBook.GetNumSpellBookSkillLines=function() C_SpellBook=SECRET;return 1 end",
        "C_SpellBook.GetNumSpellBookSkillLines=function() C_SpellBook={};return 1 end",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        let ok: bool = lua.load("return pcall(AO.collectors.spellbook)").eval()?;
        assert!(!ok, "{setup}");
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
        assert_eq!(lua.globals().get::<u32>("actions")?, 0, "{setup}");
    }
    Ok(())
}

#[test]
fn optional_spell_surfaces_omit_only_unavailable_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = fixture()?;
    lua.load("local row=AO.collectors.spellbook().entries['1'];assert(row.level_learned==20 and row.usability.usable and row.charge_count.current==1)").exec()?;
    for setup in [
        "C_Spell=SECRET",
        "C_Spell=nil",
        "C_Spell=42",
        "C_Spell.GetSpellLevelLearned=SECRET",
        "C_Spell.GetSpellLevelLearned=nil",
        "C_Spell.IsSpellUsable=SECRET",
        "C_Spell.IsSpellUsable=42",
    ] {
        lua.load("reset()").exec()?;
        lua.load(setup).exec()?;
        lua.load("local row=AO.collectors.spellbook().entries['1'];assert(row.base_spell_id==100 and row.spell_id==101)").exec()?;
        if ["C_Spell=SECRET", "C_Spell=nil", "C_Spell=42"].contains(&setup) {
            lua.load("local row=AO.collectors.spellbook().entries['1'];assert(row.level_learned==nil and row.usability==nil and row.charge_count==nil and row.reported_cast_count==nil and row.cooldown_state==nil)").exec()?;
        } else if setup.contains("GetSpellLevelLearned") {
            lua.load("local row=AO.collectors.spellbook().entries['1'];assert(row.level_learned==nil and row.usability.usable and row.charge_count.current==1)").exec()?;
        } else {
            lua.load("local row=AO.collectors.spellbook().entries['1'];assert(row.usability==nil and row.level_learned==20 and row.charge_count.current==1)").exec()?;
        }
        assert_eq!(lua.globals().get::<u32>("accesses")?, 0, "{setup}");
        assert_eq!(lua.globals().get::<u32>("actions")?, 0, "{setup}");
    }
    Ok(())
}

#[test]
fn whole_manual_capture_keeps_unrelated_equipment_and_fixed_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = fixture()?;
    lua.load("Enum=SECRET; SlashCmdList.AZEROTHOPERATOR('')")
        .exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let saved = decode(format!("AzerothOperatorExport = \"{export}\"").as_bytes())?;
    assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
    assert!(!saved.sections.contains_key("spellbook"));
    assert_eq!(saved.unavailable["spellbook"], "api_unavailable_or_failed");
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("accesses")?, 0);
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok(())
}
