use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<Lua> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};forbidden=0;combat=false
        local secret=setmetatable({}, {__index=function() error('secret container accessed') end})
        function issecretvalue(value) return rawequal(value,secret) end
        local nativeType=type
        function type(value) assert(not issecretvalue(value),'secret type inspected');return nativeType(value) end
        function InCombatLockdown() return combat end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        local function noAction() forbidden=forbidden+1;error('action forbidden') end
        CastSpellByID=noAction;TargetUnit=noAction;UseAction=noAction;PickupSpell=noAction
        Enum={SpellBookSpellBank={Player=0},SpellBookItemType={Spell=1}}
        local getterNames={charge_count='GetSpellCharges',reported_cast_count='GetSpellCastCount',cooldown_state='GetSpellCooldown'}
        function reset(field,mode,identityMode)
            calls={charge_count=0,reported_cast_count=0,cooldown_state=0};identityReads=0;timerReads=0
            local function guardFields(values)
                return setmetatable(values,{__index=function(_,key)
                    timerReads=timerReads+1
                    if key=='cooldownStartTime' or key=='cooldownDuration' or key=='chargeModRate'
                        or key=='startTime' or key=='duration' or key=='modRate' or key=='isOnGCD' then return secret end
                    error('unexpected field read: '..key)
                end})
            end
            local function normal(which)
                if which=='charge_count' then return guardFields({currentCharges=0,maxCharges=2}) end
                if which=='reported_cast_count' then return 0 end
                return guardFields({isEnabled=false,isActive=true})
            end
            local function getter(which)
                return function(id)
                    assert(id==101,'must query exact reported spell ID');calls[which]=calls[which]+1
                    local n=calls[which]
                    if which~=field then return normal(which) end
                    if mode=='error' or (mode=='second_error' and n==2) then error('getter failure') end
                    if mode=='secret' or (mode=='second_secret' and n==2) then return secret end
                    if mode=='missing' then return nil end
                    if mode=='wrong_container' then return 'wrong' end
                    if which=='reported_cast_count' then
                        if mode=='positive' then return 7 end
                        if mode=='maximum' then return 4294967295 end
                        if mode=='negative' then return -1 end
                        if mode=='fraction' then return 0.5 end
                        if mode=='overflow' then return 4294967296 end
                        if mode=='nan' then return 0/0 end
                        if mode=='infinite' then return math.huge end
                        if mode=='boolean' then return false end
                        if mode=='changed' and n==2 then return 1 end
                    elseif which=='charge_count' then
                        if mode=='positive' then return {currentCharges=1,maxCharges=2} end
                        if mode=='maximum' then return {currentCharges=4294967295,maxCharges=4294967295} end
                        if mode=='partial' then return {currentCharges=0} end
                        if mode=='secret_current' then return {currentCharges=secret,maxCharges=2} end
                        if mode=='secret_maximum' then return {currentCharges=0,maxCharges=secret} end
                        if mode=='negative' then return {currentCharges=-1,maxCharges=2} end
                        if mode=='fraction' then return {currentCharges=0.5,maxCharges=2} end
                        if mode=='overflow' then return {currentCharges=0,maxCharges=4294967296} end
                        if mode=='current_overflow' then return {currentCharges=4294967296,maxCharges=4294967295} end
                        if mode=='zero_maximum' then return {currentCharges=0,maxCharges=0} end
                        if mode=='over_maximum' then return {currentCharges=3,maxCharges=2} end
                        if mode=='boolean' then return {currentCharges=false,maxCharges=2} end
                        if mode=='changed' and n==2 then return {currentCharges=1,maxCharges=2} end
                        if mode=='changed_maximum' and n==2 then return {currentCharges=0,maxCharges=3} end
                        if mode=='changing_field' then
                            local reads=0
                            return setmetatable({maxCharges=2},{__index=function()
                                reads=reads+1;return n+reads-2
                            end})
                        end
                        if mode=='field_error' then return setmetatable({maxCharges=2},{__index=function() error('field failure') end}) end
                    else
                        if mode=='positive' then return {isEnabled=true,isActive=false} end
                        if mode=='partial' then return {isEnabled=false} end
                        if mode=='secret_enabled' then return {isEnabled=secret,isActive=false} end
                        if mode=='secret_active' then return {isEnabled=false,isActive=secret} end
                        if mode=='number' then return {isEnabled=0,isActive=true} end
                        if mode=='changed' and n==2 then return {isEnabled=true,isActive=true} end
                        if mode=='changed_active' and n==2 then return {isEnabled=false,isActive=false} end
                        if mode=='field_error' then return setmetatable({isEnabled=false},{__index=function() error('field failure') end}) end
                    end
                    return normal(which)
                end
            end
            C_Spell={GetSpellLevelLearned=function(id) assert(id==101);return 20 end,
                IsSpellUsable=function(id) assert(id==101);return true,false end,
                RequestLoadSpellData=noAction}
            for which,name in pairs(getterNames) do C_Spell[name]=getter(which) end
            if mode=='absent_getter' then C_Spell[getterNames[field]]=nil end
            if mode=='secret_getter' then C_Spell[getterNames[field]]=secret end
            if mode=='lookup_error' then
                C_Spell[getterNames[field]]=nil
                setmetatable(C_Spell,{__index=function(_,key)
                    if key==getterNames[field] then error('lookup failure') end
                end})
            end
            C_SpellBook={GetNumSpellBookSkillLines=function() return 1 end,
                GetSpellBookSkillLineInfo=function() return {shouldHide=false,isGuild=false,itemIndexOffset=2,numSpellBookItems=1} end,
                GetSpellBookItemInfo=function(slot,bank)
                    assert(slot==3 and bank==0);identityReads=identityReads+1
                    local row={itemType=1,actionID=100,spellID=101,name='Fixture spell',isPassive=false,isOffSpec=true}
                    if identityReads>1 then
                        if identityMode=='item_type' then row.itemType=2 end
                        if identityMode=='action_id' then row.actionID=102 end
                        if identityMode=='spell_id' then row.spellID=102 end
                        if identityMode=='secret' then return secret end
                        if identityMode=='secret_field' then row.spellID=secret end
                        if identityMode=='missing' then return nil end
                        if identityMode=='error' then error('recheck failure') end
                        if identityMode=='field_error' then row.spellID=nil;setmetatable(row,{__index=function() error('recheck field failure') end}) end
                    end
                    return row
                end}
            if identityMode=='lookup_error' then
                local cooldown=C_Spell.GetSpellCooldown
                C_Spell.GetSpellCooldown=function(id)
                    local value=cooldown(id)
                    if calls.cooldown_state==2 then
                        C_SpellBook.GetSpellBookItemInfo=nil
                        setmetatable(C_SpellBook,{__index=function() error('recheck lookup failure') end})
                    end
                    return value
                end
            end
        end
        reset('','normal','')
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    Ok(lua)
}

fn capture(lua: &Lua, field: &str, mode: &str, identity: &str) -> TestResult<Snapshot> {
    lua.globals()
        .get::<mlua::Function>("reset")?
        .call::<()>((field, mode, identity))?;
    lua.load("AO.Capture()").exec()?;
    let export: String = lua.globals().get("AzerothOperatorExport")?;
    let saved = decode(format!("AzerothOperatorExport = \"{export}\"").as_bytes())?;
    assert_eq!(lua.globals().get::<u32>("forbidden")?, 0);
    assert_eq!(lua.globals().get::<u32>("timerReads")?, 0);
    Ok(saved)
}

fn row(saved: &Snapshot) -> &Value {
    &saved.sections["spellbook"]["entries"]["3"]
}

fn assert_legacy(saved: &Snapshot) {
    let row = row(saved);
    assert_eq!(row["base_spell_id"], 100);
    assert_eq!(row["spell_id"], 101);
    assert_eq!(row["name"], "Fixture spell");
    assert_eq!(row["passive"], false);
    assert_eq!(row["off_spec"], true);
    assert_eq!(row["level_learned"], 20);
    assert_eq!(
        row["usability"],
        json!({"usable":true,"insufficient_power":false})
    );
    assert_eq!(saved.sections["equipment"]["16"]["item_id"], 123);
}

#[test]
fn public_spell_resource_primitives_round_trip_through_core_and_import() -> TestResult {
    let lua = fixture()?;
    let saved = capture(&lua, "", "normal", "")?;
    assert_legacy(&saved);
    assert_eq!(
        row(&saved)["charge_count"],
        json!({"current":0,"maximum":2})
    );
    assert_eq!(row(&saved)["reported_cast_count"], 0);
    assert_eq!(
        row(&saved)["cooldown_state"],
        json!({"enabled":false,"active":true})
    );
    for (field, mode, expected) in [
        ("charge_count", "positive", json!({"current":1,"maximum":2})),
        (
            "charge_count",
            "maximum",
            json!({"current":4294967295u64,"maximum":4294967295u64}),
        ),
        ("reported_cast_count", "positive", json!(7)),
        ("reported_cast_count", "maximum", json!(4294967295u64)),
        (
            "cooldown_state",
            "positive",
            json!({"enabled":true,"active":false}),
        ),
    ] {
        let saved = capture(&lua, field, mode, "")?;
        assert_eq!(row(&saved)[field], expected, "{field}/{mode}");
        assert_legacy(&saved);
    }
    lua.load("assert(calls.charge_count==2 and calls.reported_cast_count==2 and calls.cooldown_state==2 and identityReads==2)").exec()?;
    Ok(())
}

#[test]
fn optional_resource_failures_remain_independent_and_preserve_legacy_metadata() -> TestResult {
    let lua = fixture()?;
    let fields = ["charge_count", "reported_cast_count", "cooldown_state"];
    for field in fields {
        let mut modes = vec![
            "error",
            "second_error",
            "secret",
            "second_secret",
            "missing",
            "wrong_container",
            "changed",
            "absent_getter",
            "secret_getter",
            "lookup_error",
        ];
        modes.extend(match field {
            "charge_count" => vec![
                "partial",
                "secret_current",
                "secret_maximum",
                "negative",
                "fraction",
                "overflow",
                "current_overflow",
                "zero_maximum",
                "over_maximum",
                "boolean",
                "changed_maximum",
                "field_error",
                "changing_field",
            ],
            "reported_cast_count" => vec![
                "negative", "fraction", "overflow", "nan", "infinite", "boolean",
            ],
            _ => vec![
                "partial",
                "secret_enabled",
                "secret_active",
                "number",
                "changed_active",
                "field_error",
            ],
        });
        for mode in modes {
            let saved = capture(&lua, field, mode, "")?;
            assert_legacy(&saved);
            for observed in fields {
                assert_eq!(
                    row(&saved).get(observed).is_some(),
                    observed != field,
                    "{field}/{mode}: {observed}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn changed_or_unreadable_spell_identity_omits_only_new_resources() -> TestResult {
    let lua = fixture()?;
    for identity in [
        "item_type",
        "action_id",
        "spell_id",
        "secret",
        "secret_field",
        "missing",
        "error",
        "field_error",
        "lookup_error",
    ] {
        let saved = capture(&lua, "", "normal", identity)?;
        assert_legacy(&saved);
        for field in ["charge_count", "reported_cast_count", "cooldown_state"] {
            assert!(row(&saved).get(field).is_none(), "{identity}: {field}");
        }
    }
    Ok(())
}

#[test]
fn capture_stays_passive_and_combat_and_other_families_do_not_query_resources() -> TestResult {
    let lua = fixture()?;
    capture(&lua, "", "normal", "")?;
    lua.load(r#"
        reset('','normal','');combat=true
        local before=AzerothOperatorExport
        assert(not pcall(AO.Capture));assert(AzerothOperatorExport==before)
        assert(calls.charge_count==0 and calls.reported_cast_count==0 and calls.cooldown_state==0)
        combat=false;AO.family='retail'
        local observed=AO.collectors.spellbook().entries['3']
        assert(observed.spell_id==101 and observed.level_learned==20 and observed.usability.usable)
        assert(observed.charge_count==nil and observed.reported_cast_count==nil and observed.cooldown_state==nil)
        assert(calls.charge_count==0 and calls.reported_cast_count==0 and calls.cooldown_state==0)
        assert(forbidden==0 and timerReads==0)
    "#).exec()?;
    Ok(())
}
