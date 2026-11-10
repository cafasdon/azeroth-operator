use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn saved_instances_wait_for_update_and_omit_private_ids() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; mode='normal'; now=1735689600; counts=0
        function CreateFrame()
            return {RegisterEvent=function(self,event) self[event]=true end,
                SetScript=function(self,_,callback) self.callback=callback end}
        end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return now end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(value) return value=='PRIVATE-LOCKOUT' or (mode=='secret_name' and value=='Raid') end
        function GetNumSavedInstances()
            counts=counts+1
            if mode=='empty' then return 0 end
            if mode=='oversized' then return 129 end
            if mode=='changed' and counts>1 then return 2 end
            return 1
        end
        function GetSavedInstanceInfo()
            local reset=mode=='negative' and -1 or (mode=='bad_reset' and 'bad' or 123.25)
            local progress=mode=='secret_progress' and 'PRIVATE-LOCKOUT' or (mode=='bad_progress' and 11 or 4)
            if mode=='missing_progress' then progress=nil end
            return 'Raid','PRIVATE-LOCKOUT',reset,3,true,false,'PRIVATE-LOCKOUT',true,40,'40 Player',10,progress
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
    assert!(capture()?.unavailable.contains_key("saved_instances"));
    lua.load("AO.savedInstanceObserver:callback('UPDATE_INSTANCE_INFO'); now=now+5")
        .exec()?;
    let saved = capture()?;
    assert_eq!(saved.sections["saved_instances"]["update_age_seconds"], 5);
    assert_eq!(
        saved.sections["saved_instances"]["entries"]["1"]["reset_seconds"],
        124
    );
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE-LOCKOUT"));
    let row = &saved.sections["saved_instances"]["entries"]["1"];
    assert_eq!(row["encounter_total"], 10);
    assert_eq!(row["reported_encounter_progress"], 4);
    for mode in ["secret_progress", "bad_progress", "missing_progress"] {
        lua.globals().set("mode", mode)?;
        let snapshot = capture()?;
        let row = &snapshot.sections["saved_instances"]["entries"]["1"];
        assert_eq!(row["name"], "Raid", "{mode}");
        assert!(row.get("encounter_total").is_none(), "{mode}");
        assert!(row.get("reported_encounter_progress").is_none(), "{mode}");
    }
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["locked_instances_observed"],
        1
    );
    lua.load("mode='negative'").exec()?;
    assert_eq!(
        capture()?.sections["saved_instances"]["entries"]["1"]["reset_seconds"],
        -1
    );
    lua.load("mode='empty'").exec()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&capture()?)["saved_instances_observed"],
        0
    );
    for mode in ["oversized", "changed", "bad_reset", "secret_name"] {
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?.unavailable.contains_key("saved_instances"),
            "{mode}"
        );
    }
    lua.load("mode='normal';AO.savedInstanceObserver:callback('PLAYER_ENTERING_WORLD')")
        .exec()?;
    assert!(capture()?.unavailable.contains_key("saved_instances"));
    Ok(())
}
