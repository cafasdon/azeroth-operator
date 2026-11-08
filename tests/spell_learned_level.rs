use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn spell_learned_levels_preserve_zero_and_unknowns() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';queries=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={SpellBookSpellBank={Player=0},SpellBookItemType={Spell=1}}
        C_SpellBook={GetNumSpellBookSkillLines=function() return 1 end,
          GetSpellBookSkillLineInfo=function() return {shouldHide=false,isGuild=false,itemIndexOffset=2,numSpellBookItems=1} end,
          GetSpellBookItemInfo=function(slot,bank) assert(slot==3 and bank==0);return {itemType=1,actionID=100,spellID=101} end}
        C_Spell={RequestLoadSpellData=function() error('must not request loading') end,
          GetSpellLevelLearned=function(id)
            assert(id==101);queries=queries+1
            if mode=='zero' then return 0 end
            if mode=='maximum' then return 4294967295 end
            if mode=='secret' or (mode=='second_secret' and queries>1) then return 'PRIVATE' end
            if mode=='missing' then return nil end
            if mode=='error' or (mode=='second_error' and queries>1) then error('PRIVATE') end
            if mode=='negative' then return -1 end
            if mode=='fraction' then return 1.5 end
            if mode=='overflow' then return 4294967296 end
            if mode=='changed' and queries>1 then return 21 end
            return 20
          end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("queries=0;AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    for (mode, expected) in [("normal", 20u64), ("zero", 0), ("maximum", 4294967295)] {
        lua.globals().set("mode", mode)?;
        assert_eq!(
            capture()?.sections["spellbook"]["entries"]["3"]["level_learned"],
            expected
        );
        assert_eq!(lua.globals().get::<u32>("queries")?, 2);
    }
    for mode in [
        "secret",
        "second_secret",
        "missing",
        "error",
        "second_error",
        "negative",
        "fraction",
        "overflow",
        "changed",
    ] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(
            saved.sections["spellbook"]["entries"]["3"]
                .get("level_learned")
                .is_none(),
            "{mode}"
        );
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for statement in ["C_Spell={}", "C_Spell=nil"] {
        lua.load(statement).exec()?;
        assert!(
            capture()?.sections["spellbook"]["entries"]["3"]
                .get("level_learned")
                .is_none()
        );
    }
    Ok(())
}
