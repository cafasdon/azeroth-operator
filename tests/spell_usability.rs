use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn observed_spell_usability_is_optional_readable_and_stable()
-> Result<(), Box<dyn std::error::Error>> {
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
        function CastSpellByID() error('casting forbidden') end
        function TargetUnit() error('targeting forbidden') end
        Enum={SpellBookSpellBank={Player=0},SpellBookItemType={Spell=1}}
        C_SpellBook={GetNumSpellBookSkillLines=function() return 1 end,
            GetSpellBookSkillLineInfo=function() return {shouldHide=false,isGuild=false,itemIndexOffset=2,numSpellBookItems=1} end,
            GetSpellBookItemInfo=function(slot,bank) assert(slot==3 and bank==0);return {itemType=1,actionID=100,spellID=101} end}
        C_Spell={GetSpellLevelLearned=function(id) assert(id==101);return 20 end,
            RequestLoadSpellData=function() error('load forbidden') end,
            IsSpellUsable=function(id)
                assert(id==101);queries=queries+1
                if mode=='blocked' then return false,true end
                if mode=='false_pair' then return false,false end
                if mode=='secret_usable' then return 'PRIVATE',false end
                if mode=='secret_power' then return true,'PRIVATE' end
                if mode=='missing' then return end
                if mode=='partial' then return false end
                if mode=='number' then return 1,false end
                if mode=='error' or (mode=='second_error' and queries>1) then error('PRIVATE') end
                if mode=='second_secret' and queries>1 then return false,'PRIVATE' end
                if mode=='changed' and queries>1 then return false,false end
                if mode=='changed_power' and queries>1 then return true,true end
                return true,false
            end}
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("queries=0;AO.Capture()").exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    for (mode, usable, power) in [
        ("normal", true, false),
        ("blocked", false, true),
        ("false_pair", false, false),
    ] {
        let saved = capture(mode)?;
        assert_eq!(
            saved.sections["spellbook"]["entries"]["3"]["usability"],
            serde_json::json!({"usable":usable,"insufficient_power":power})
        );
        assert_eq!(
            saved.sections["spellbook"]["entries"]["3"]["level_learned"],
            20
        );
    }
    for mode in [
        "secret_usable",
        "secret_power",
        "missing",
        "partial",
        "number",
        "error",
        "second_error",
        "second_secret",
        "changed",
        "changed_power",
    ] {
        let saved = capture(mode)?;
        assert!(
            saved.sections["spellbook"]["entries"]["3"]
                .get("usability")
                .is_none(),
            "{mode}"
        );
        assert_eq!(saved.sections["spellbook"]["entries"]["3"]["spell_id"], 101);
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    lua.load("C_Spell.IsSpellUsable=nil").exec()?;
    assert!(
        capture("normal")?.sections["spellbook"]["entries"]["3"]
            .get("usability")
            .is_none()
    );
    Ok(())
}
