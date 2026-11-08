use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn trainer_profession_flags_and_slot_observations_are_guarded()
-> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';profCalls=0;infoCalls=0;costCalls={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={TrainerType={General=1,Tradeskills=2,Pet=3}}
        C_Trainer={GetTrainerType=function() return mode=='pet' and 3 or 1 end}
        ClassTrainerFrame={IsVisible=function() return true end}
        function GetProfessions()
            profCalls=profCalls+1
            if mode=='free' or (mode=='changed_slots' and profCalls>1) then return 1,nil end
            if mode=='empty' then return nil,nil end
            if mode=='secret_slots' then return 'PRIVATE',2 end
            if mode=='sparse_slots' then return nil,2 end
            if mode=='duplicate_slots' then return 1,1 end
            return 1,2
        end
        function GetNumTrainerServices() return 3 end
        function GetTrainerServiceInfo(index)
            infoCalls=infoCalls+1
            if index==1 then return mode=='changed_name' and infoCalls>3 and 'Changed' or 'Profession','available',nil,5,'Rank 1' end
            if index==2 then return 'Lesson','unavailable',nil,10 end
            return 'Known','used'
        end
        function GetTrainerServiceCost(index)
            assert(index~=3);costCalls[index]=(costCalls[index] or 0)+1
            if mode=='cost_error' then error('PRIVATE') end
            local flag=index==1
            if mode=='secret_flag' then flag='PRIVATE' end
            if mode=='changed_flag' and costCalls[index]>1 then flag=not flag end
            return mode=='changed_price' and costCalls[index]>1 and 200 or 100,flag
        end
        function GetTrainerServiceNumAbilityReq() return 0 end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("profCalls=0;infoCalls=0;costCalls={};AO.Capture()")
            .exec()?;
        let text: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{text}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["trainer_services"]["primary_profession_slots_full"],
        true
    );
    assert_eq!(
        saved.sections["trainer_services"]["entries"]["1"]["is_profession"],
        true
    );
    assert_eq!(
        saved.sections["trainer_services"]["entries"]["2"]["is_profession"],
        false
    );
    assert!(
        saved.sections["trainer_services"]["entries"]["3"]
            .get("is_profession")
            .is_none()
    );
    for mode in ["free", "empty"] {
        lua.globals().set("mode", mode)?;
        assert_eq!(
            capture()?.sections["trainer_services"]["primary_profession_slots_full"],
            false
        );
    }
    for mode in [
        "secret_slots",
        "sparse_slots",
        "duplicate_slots",
        "changed_slots",
    ] {
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?.sections["trainer_services"]
                .get("primary_profession_slots_full")
                .is_none()
        );
    }
    for mode in ["secret_flag", "cost_error"] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(
            saved.sections["trainer_services"]["entries"]["1"]
                .get("is_profession")
                .is_none()
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in ["changed_flag", "changed_name", "changed_price"] {
        lua.globals().set("mode", mode)?;
        let saved = capture()?;
        assert!(saved.unavailable.contains_key("trainer_services"), "{mode}");
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.load("mode='pet'").exec()?;
    let saved = capture()?;
    assert!(
        saved.sections["trainer_services"]
            .get("primary_profession_slots_full")
            .is_none()
    );
    assert!(
        saved.sections["trainer_services"]["entries"]["1"]
            .get("is_profession")
            .is_none()
    );
    assert_eq!(lua.globals().get::<u32>("profCalls")?, 0);
    lua.load("mode='normal';GetProfessions=nil").exec()?;
    assert!(
        capture()?.sections["trainer_services"]
            .get("primary_profession_slots_full")
            .is_none()
    );
    Ok(())
}
