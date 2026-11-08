use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn world_boss_cache_requires_update_and_omits_ids() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'; now=1735689600; counts=0
        function CreateFrame() return {RegisterEvent=function() end,
            SetScript=function(self,_,callback) self.callback=callback end} end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return now end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return value=='OMITTED-ID' or (mode=='secret' and value=='Boss') end
        function GetNumSavedWorldBosses()
            counts=counts+1
            if mode=='empty' then return 0 end
            if mode=='oversized' then return 129 end
            if mode=='changed' and counts>1 then return 2 end
            return 1
        end
        function GetSavedWorldBossInfo()
            if mode=='invalid' then return 'Boss','OMITTED-ID',0/0 end
            if mode=='event' then AO.savedInstanceObserver:callback('PLAYER_ENTERING_WORLD') end
            return 'Boss','OMITTED-ID',mode=='negative' and -1 or 123.25
        end
        function RequestRaidInfo() error('refresh forbidden') end
        function SetSavedInstanceExtend() error('extension forbidden') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("counts=0;AO.Capture()").exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    assert!(capture()?.unavailable.contains_key("saved_world_bosses"));
    lua.load("AO.savedInstanceObserver:callback('UPDATE_INSTANCE_INFO');now=now+5")
        .exec()?;
    let saved = capture()?;
    assert_eq!(
        saved.sections["saved_world_bosses"]["update_age_seconds"],
        5
    );
    assert_eq!(
        saved.sections["saved_world_bosses"]["entries"]["1"]["reset_seconds"],
        124
    );
    assert!(!serde_json::to_string(&saved)?.contains("OMITTED-ID"));
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["saved_world_bosses_observed"],
        1
    );
    lua.load("mode='negative'").exec()?;
    assert_eq!(
        capture()?.sections["saved_world_bosses"]["entries"]["1"]["reset_seconds"],
        -1
    );
    lua.load("mode='empty'").exec()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&capture()?)["saved_world_bosses_observed"],
        0
    );
    for mode in ["oversized", "changed", "secret", "invalid", "event"] {
        lua.load("AO.savedInstanceObserver:callback('UPDATE_INSTANCE_INFO')")
            .exec()?;
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?.unavailable.contains_key("saved_world_bosses"),
            "{mode}"
        );
    }
    Ok(())
}
