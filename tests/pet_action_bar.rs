use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn pet_actions_are_read_only_partial_and_context_bound() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';queries=0;reads={};NUM_PET_ACTION_SLOTS=10
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function issecretvalue(v) return v=='PRIVATE' end
        function UnitExists(unit)
            assert(unit=='pet')
            if mode=='secret_context' then return 'PRIVATE' end
            return mode~='no_pet' and mode~='mismatch' and not(mode=='vanished' and queries>0)
        end
        function PetHasActionBar() return mode~='no_pet' and mode~='no_bar' and not(mode=='bar_changed' and queries>0) end
        function GetPetActionInfo(slot)
            assert(mode~='no_pet' and mode~='no_bar');assert(slot>=1 and slot<=10)
            queries=queries+1;reads[slot]=(reads[slot] or 0)+1
            if slot==1 then return mode=='secret_command' and 'PRIVATE' or mode=='bad_command' and 'Attack' or 'PET_ACTION_ATTACK',
                'PRIVATE',true,false,false,false,nil,false,'PRIVATE' end
            if slot==2 then
                if mode=='error' then error('PRIVATE') end
                if mode=='changed_kind' and reads[slot]>1 then return 'PET_MODE_DEFENSIVE','PRIVATE',true,false end
                local id=mode=='secret_id' and 'PRIVATE' or mode=='invalid_id' and true or (mode=='changed_id' and reads[slot]>1) and 102 or 101
                if mode=='secret_flags' then return 'PRIVATE','PRIVATE',false,'PRIVATE','PRIVATE','PRIVATE',id end
                return 'PRIVATE','PRIVATE',false,true,true,not(mode=='changed_flag' and reads[slot]>1),id,true,true
            end
            if slot==10 then
                if mode=='no_range_check' then return 'PRIVATE','PRIVATE',false,nil,nil,nil,102,false,'PRIVATE' end
                return 'PRIVATE','PRIVATE',false,nil,nil,nil,102,true,
                    mode=='secret_range' and 'PRIVATE' or (mode=='changed_range' and reads[slot]>1) or false
            end
        end
        function CastPetAction() error('action forbidden') end
        function TogglePetAutocast() error('toggle forbidden') end
        function PetAttack() error('attack forbidden') end
        function PetFollow() error('follow forbidden') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = |mode: &str| -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.globals().set("mode", mode)?;
        lua.load("queries=0;reads={};AO.Capture()").exec()?;
        let export: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{export}\"").as_bytes(),
        )?)
    };
    let saved = capture("normal")?;
    let rows = &saved.sections["pet_action_bar"]["entries"];
    assert_eq!(rows.as_object().unwrap().len(), 3);
    assert_eq!(rows["1"]["command_token"], "PET_ACTION_ATTACK");
    assert_eq!(rows["1"]["is_active"], false);
    assert_eq!(rows["2"]["spell_id"], 101);
    assert_eq!(rows["2"]["autocast_enabled"], true);
    assert_eq!(rows["10"]["in_range"], false);
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    for (mode, missing) in [
        ("secret_command", "1"),
        ("bad_command", "1"),
        ("secret_id", "2"),
        ("invalid_id", "2"),
        ("changed_id", "2"),
        ("changed_kind", "2"),
        ("error", "2"),
    ] {
        let saved = capture(mode)?;
        assert!(
            saved.sections["pet_action_bar"]["entries"]
                .get(missing)
                .is_none(),
            "{mode}"
        );
        assert_eq!(
            saved.sections["pet_action_bar"]["entries"]["10"]["spell_id"],
            102
        );
        assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    }
    for mode in ["secret_flags", "changed_flag"] {
        let saved = capture(mode)?;
        assert!(
            saved.sections["pet_action_bar"]["entries"]["2"]
                .get("autocast_enabled")
                .is_none()
        );
        assert_eq!(
            saved.sections["pet_action_bar"]["entries"]["2"]["spell_id"],
            101
        );
    }
    for mode in ["secret_range", "changed_range"] {
        let saved = capture(mode)?;
        assert!(
            saved.sections["pet_action_bar"]["entries"]["10"]
                .get("range_check_performed")
                .is_none()
        );
    }
    assert_eq!(
        capture("no_range_check")?.sections["pet_action_bar"]["entries"]["10"]["range_check_performed"],
        false
    );
    for mode in ["no_pet", "no_bar"] {
        let saved = capture(mode)?;
        assert_eq!(saved.sections["pet_action_bar"]["has_action_bar"], false);
        assert_eq!(
            saved.sections["pet_action_bar"]["entries"],
            serde_json::json!({})
        );
    }
    for mode in ["secret_context", "mismatch", "vanished", "bar_changed"] {
        let saved = capture(mode)?;
        assert!(saved.unavailable.contains_key("pet_action_bar"), "{mode}");
        assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
    }
    lua.load("GetPetActionInfo=nil").exec()?;
    assert!(
        capture("normal")?
            .unavailable
            .contains_key("pet_action_bar")
    );
    assert!(capture("no_pet")?.sections.contains_key("pet_action_bar"));
    lua.load("NUM_PET_ACTION_SLOTS=nil").exec()?;
    assert!(capture("no_pet")?.sections.contains_key("pet_action_bar"));
    assert!(capture("no_bar")?.sections.contains_key("pet_action_bar"));
    Ok(())
}
