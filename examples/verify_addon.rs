//! Execute an actual staged addon in a Lua 5.1 fixture; never a live compatibility claim.
use azeroth_operator::import::decode;
use mlua::Lua;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "Usage: verify_addon <directory> <family> <variant> <interface>"
    );
    let interface: u32 = args[3].parse()?;
    let lua = Lua::new();
    lua.globals()
        .set("expected_interface", interface)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let fixture = r#"
        AO={}; SlashCmdList={}
        function InCombatLockdown() return false end
        function GetBuildInfo() return "fixture", "1", "", expected_interface end
        function GetInventoryItemID(_,slot) if slot==16 then return 123 end end
        function GetInventoryItemLink() return nil end
        function UnitLevel() return 1 end
        function UnitClass() return "Fixture", "WARRIOR" end
        function UnitRace() return "Fixture", "Human" end
        function UnitName() return "PackageFixture" end
        function GetRealmName() return "Synthetic" end
        function GetServerTime() return 1735689600 end
        function date() return "2025-01-01T00:00:00Z" end
        setName='Package set'; setIcon=134400; setWrites=0
        C_EquipmentSet={CanUseEquipmentSets=function() return true end,
            GetEquipmentSetIDs=function() return {0} end,GetNumEquipmentSets=function() return 1 end,
            GetEquipmentSetInfo=function(id) return setName,setIcon,id,false,0,0,0,0,0 end,
            GetIgnoredSlots=function() return {} end,GetItemIDs=function() return {} end,
            GetItemLocations=function() return {} end,GetEquipmentSetAssignedSpec=function() end,
            GetEquipmentSetID=function() end,
            ModifyEquipmentSet=function(id,name,icon)
                assert(id==0 and AzerothOperatorOperations:find(':outcome_unknown:modify_equipment_set;',1,true))
                setWrites=setWrites+1; setName=name; setIcon=tonumber(icon)
            end}
        function GetCursorInfo() end; function UnitCastingInfo() end; function UnitChannelInfo() end
    "#;
    let result = (|| -> mlua::Result<String> {
        lua.load(fixture).exec()?;
        let namespace: mlua::Table = lua.globals().get("AO")?;
        for name in [
            "Adapter.lua",
            "Collectors.lua",
            "Core.lua",
            "MacroOperations.lua",
            "BindingOperations.lua",
            "BindingImport.lua",
            "EquipmentOperations.lua",
            "EquipmentImport.lua",
            "BagOperations.lua",
            "BagImport.lua",
            "TalentOperations.lua",
            "TalentImport.lua",
            "ActionOperations.lua",
            "ActionImport.lua",
            "EquipmentSetOperations.lua",
            "EquipmentSetUse.lua",
            "EquipmentSetImport.lua",
            "EquipmentSetSpec.lua",
            "EquipmentSetSpecImport.lua",
            "EquipmentSetSaveState.lua",
            "EquipmentSetSave.lua",
            "EquipmentSetSaveImport.lua",
            "MacroImport.lua",
            "MacroUI.lua",
        ] {
            let source = std::fs::read_to_string(Path::new(&args[0]).join(name))
                .map_err(mlua::Error::external)?;
            lua.load(&source)
                .call::<()>(("AzerothOperator", namespace.clone()))?;
        }
        lua.load("AO.Capture()").exec()?;
        let previous: String = lua.globals().get("AzerothOperatorExport")?;
        lua.load("expected_interface=expected_interface+1; assert(not pcall(AO.Capture))")
            .exec()?;
        assert_eq!(
            previous,
            lua.globals().get::<String>("AzerothOperatorExport")?
        );
        Ok(previous)
    })()
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let snapshot = decode(format!("AzerothOperatorExport = \"{result}\"").as_bytes())?;
    anyhow::ensure!(
        serde_json::to_value(&snapshot.client.family)? == args[1],
        "Family mismatch"
    );
    anyhow::ensure!(
        snapshot.client.variant == args[2] && snapshot.client.interface == interface,
        "Client mismatch"
    );
    anyhow::ensure!(
        snapshot.sections["equipment"]["16"]["item_id"] == 123,
        "Item observation mismatch"
    );
    if args[1] == "retail" {
        let prepared = azeroth_operator::equipment_set_operation::compile(
            &snapshot,
            &"a".repeat(64),
            &azeroth_operator::equipment_set_operation::EquipmentSetRequest {
                operation: azeroth_operator::equipment_set_operation::EquipmentSetAction::Modify,
                set_id: Some(0),
                name: Some("Verified package".into()),
                icon: Some(123),
                expect_empty_slots: vec![],
                spec_index: None,
                ignored_slots: None,
            },
        )?;
        let receipt_export=(|| -> mlua::Result<String> {
            lua.globals().set("expected_interface",interface)?;
            lua.globals().set("setCode",prepared["import_code"].as_str().unwrap())?;
            lua.load(r#"
                assert(not pcall(AO.ApplyReviewedMacro,setCode) and setWrites==0)
                AO.ReviewMacroImport(setCode)
                local receipt,ok=AO.ApplyReviewedMacro(setCode)
                assert(ok and receipt.status=='applied' and setName=='Verified package' and setIcon==123 and setWrites==1)
                AO.ReviewMacroImport(setCode); assert(not pcall(AO.ApplyReviewedMacro,setCode) and setWrites==1)
                AO.Capture()
            "#).exec()?;
            lua.globals().get("AzerothOperatorExport")
        })().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let verified = decode(format!("AzerothOperatorExport = \"{receipt_export}\"").as_bytes())?;
        anyhow::ensure!(
            verified.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]["status"]
                == "applied",
            "Packaged set receipt missing"
        );
        let use_capture = (|| -> mlua::Result<String> {
            lua.load(r#"
                wornGUID='Item-old'; useWrites=0; NUM_TOTAL_EQUIPPED_BAG_SLOTS=0
                ItemLocation={CreateFromEquipmentSlot=function(_,slot) return {slot=slot} end,
                    CreateFromBagAndSlot=function(_,bag,slot) return {bag=bag,slot=slot} end}
                C_Item={GetItemGUID=function(loc) if loc.bag~=nil then return 'Item-target' end if loc.slot==16 then return wornGUID end end,
                    GetItemID=function(loc) if loc.bag~=nil or loc.slot==16 then return 123 end end,
                    IsLocked=function() return false end,
                    DoesItemExist=function(loc) return loc.slot==16 end}
                C_Container={GetContainerNumSlots=function() return 1 end}
                function IsInventoryItemLocked() return false end
                function EquipmentManager_GetLocationData()
                    return {isPlayer=true,isBank=false,isBags=true,bag=0,slot=1}
                end
                C_EquipmentSet.GetEquipmentSetInfo=function(id) return setName,setIcon,id,false,1,0,1,0,18 end
                C_EquipmentSet.GetIgnoredSlots=function()
                    local result={} for i=1,19 do result[i]=i~=16 end return result
                end
                C_EquipmentSet.GetItemIDs=function() return {[16]=123} end
                C_EquipmentSet.GetItemLocations=function() return {[16]=100} end
                C_EquipmentSet.EquipmentSetContainsLockedItems=function() return false end
                C_EquipmentSet.UseEquipmentSet=function(id)
                    assert(id==0 and AzerothOperatorOperations:find(':outcome_unknown:use_equipment_set;',1,true))
                    useWrites=useWrites+1; wornGUID='Item-target'
                end
                AO.Capture()
            "#).exec()?;
            lua.globals().get("AzerothOperatorExport")
        })().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let use_snapshot = decode(format!("AzerothOperatorExport = \"{use_capture}\"").as_bytes())?;
        let use_op = azeroth_operator::equipment_set_operation::compile(
            &use_snapshot,
            &"b".repeat(64),
            &azeroth_operator::equipment_set_operation::EquipmentSetRequest {
                operation: azeroth_operator::equipment_set_operation::EquipmentSetAction::Use,
                set_id: Some(0),
                name: None,
                icon: None,
                expect_empty_slots: (1..=19).filter(|i| *i != 16).collect(),
                spec_index: None,
                ignored_slots: None,
            },
        )?;
        let use_export=(|| -> mlua::Result<String> {
            lua.globals().set("useCode",use_op["import_code"].as_str().unwrap())?;
            lua.load(r#"
                AO.ReviewMacroImport(useCode)
                local receipt,ok=AO.ApplyReviewedMacro(useCode)
                assert(ok and receipt.status=='applied' and useWrites==1 and wornGUID=='Item-target')
                AO.ReviewMacroImport(useCode); assert(not pcall(AO.ApplyReviewedMacro,useCode) and useWrites==1)
                AO.Capture()
            "#).exec()?;
            lua.globals().get("AzerothOperatorExport")
        })().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let use_verified = decode(format!("AzerothOperatorExport = \"{use_export}\"").as_bytes())?;
        anyhow::ensure!(
            use_verified.sections["operation_receipts"][use_op["operation_id"].as_str().unwrap()]["status"]
                == "applied",
            "Packaged set use receipt missing"
        );
        lua.load(r#"
                association=nil; associationWrites=0
                function GetNumSpecializations() return 3 end
                C_SpecializationInfo={GetSpecializationInfo=function(i) return 100+i end}
                C_EquipmentSet.GetEquipmentSetAssignedSpec=function() return association end
                C_EquipmentSet.GetEquipmentSetForSpec=function(spec) if association==spec then return 0 end end
                C_EquipmentSet.AssignSpecToEquipmentSet=function(id,spec)
                    assert(id==0 and AzerothOperatorOperations:find(':outcome_unknown:assign_equipment_set_spec;',1,true))
                    associationWrites=associationWrites+1; association=spec
                end
                C_EquipmentSet.UnassignEquipmentSetSpec=function(id)
                    assert(id==0 and AzerothOperatorOperations:find(':outcome_unknown:unassign_equipment_set_spec;',1,true))
                    associationWrites=associationWrites+1; association=nil
                end
            "#).exec().map_err(|e|anyhow::anyhow!(e.to_string()))?;
        for assigning in [true, false] {
            let captured = (|| -> mlua::Result<String> {
                lua.load("AO.Capture()").exec()?;
                lua.globals().get("AzerothOperatorExport")
            })()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let snap = decode(format!("AzerothOperatorExport = \"{captured}\"").as_bytes())?;
            let request = azeroth_operator::equipment_set_operation::EquipmentSetRequest {
                operation: if assigning {
                    azeroth_operator::equipment_set_operation::EquipmentSetAction::AssignSpec
                } else {
                    azeroth_operator::equipment_set_operation::EquipmentSetAction::UnassignSpec
                },
                set_id: Some(0),
                name: None,
                icon: None,
                expect_empty_slots: vec![],
                spec_index: assigning.then_some(1),
                ignored_slots: None,
            };
            let prepared = azeroth_operator::equipment_set_operation::compile(
                &snap,
                &"c".repeat(64),
                &request,
            )?;
            let exported=(|| -> mlua::Result<String> {
                lua.globals().set("associationCode",prepared["import_code"].as_str().unwrap())?;
                lua.load(r#"
                    local before=associationWrites
                    AO.ReviewMacroImport(associationCode)
                    local receipt,ok=AO.ApplyReviewedMacro(associationCode)
                    assert(ok and receipt.status=='applied' and associationWrites==before+1)
                    AO.ReviewMacroImport(associationCode)
                    assert(not pcall(AO.ApplyReviewedMacro,associationCode) and associationWrites==before+1)
                    AO.Capture()
                "#).exec()?;
                lua.globals().get("AzerothOperatorExport")
            })().map_err(|e|anyhow::anyhow!(e.to_string()))?;
            let verified = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
            anyhow::ensure!(
                verified.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]
                    ["status"]
                    == "applied",
                "Packaged association receipt missing"
            );
            anyhow::ensure!(
                verified.sections["equipment_sets"]["entries"]["0"]
                    .get("assigned_spec")
                    .and_then(serde_json::Value::as_u64)
                    == assigning.then_some(1),
                "Packaged association state mismatch"
            );
        }
    }
    if args[1] == "retail" {
        lua.load(r#"
            savedName=nil; savedIcon=nil; saveWrites=0; saveFlags={}; savedFlags={}
            for i=1,19 do saveFlags[i]=false end
            MAX_EQUIPMENT_SETS_PER_PLAYER=20
            function StaticPopup_Visible() end
            function EquipmentManager_GetLocationData(location) return {isPlayer=true,isBank=false,isBags=false,slot=location} end
            C_EquipmentSet.GetEquipmentSetIDs=function() return savedName and {0} or {} end
            C_EquipmentSet.GetNumEquipmentSets=function() return savedName and 1 or 0 end
            C_EquipmentSet.GetEquipmentSetID=function(name) if savedName==name then return 0 end end
            C_EquipmentSet.GetEquipmentSetInfo=function(id) return savedName,savedIcon,id,true,1,1,0,0,18 end
            C_EquipmentSet.GetIgnoredSlots=function() return savedFlags end
            C_EquipmentSet.GetItemLocations=function() return {[16]=16} end
            C_EquipmentSet.GetEquipmentSetAssignedSpec=function() end
            C_EquipmentSet.IsSlotIgnoredForSave=function(i) return saveFlags[i] end
            C_EquipmentSet.IgnoreSlotForSave=function(i) saveFlags[i]=true end
            C_EquipmentSet.UnignoreSlotForSave=function(i) saveFlags[i]=false end
            local function saveFlagsNow()
                for i=1,19 do savedFlags[i]=saveFlags[i]; assert(saveFlags[i]==(i~=16)) end
                saveWrites=saveWrites+1
            end
            C_EquipmentSet.CreateEquipmentSet=function(name,icon)
                assert(AzerothOperatorOperations:find(':outcome_unknown:create_equipment_set;',1,true))
                savedName=name; savedIcon=tonumber(icon); saveFlagsNow()
            end
            C_EquipmentSet.SaveEquipmentSet=function(id,icon)
                assert(id==0 and AzerothOperatorOperations:find(':outcome_unknown:save_equipment_set;',1,true))
                savedIcon=tonumber(icon); saveFlagsNow()
            end
        "#).exec().map_err(|e|anyhow::anyhow!(e.to_string()))?;
        for creating in [true, false] {
            let captured = (|| -> mlua::Result<String> {
                lua.load("AO.Capture()").exec()?;
                lua.globals().get("AzerothOperatorExport")
            })()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let snap = decode(format!("AzerothOperatorExport = \"{captured}\"").as_bytes())?;
            let prepared = azeroth_operator::equipment_set_operation::compile(
                &snap,
                &"d".repeat(64),
                &azeroth_operator::equipment_set_operation::EquipmentSetRequest {
                    operation: if creating {
                        azeroth_operator::equipment_set_operation::EquipmentSetAction::Create
                    } else {
                        azeroth_operator::equipment_set_operation::EquipmentSetAction::Save
                    },
                    set_id: if creating { None } else { Some(0) },
                    name: creating.then(|| "Package saved set".into()),
                    icon: Some(if creating { 123 } else { 124 }),
                    expect_empty_slots: (1..=19).filter(|i| *i != 16).collect(),
                    ignored_slots: Some((1..=19).filter(|i| *i != 16).collect()),
                    spec_index: None,
                },
            )?;
            let exported=(|| -> mlua::Result<String> {
                lua.globals().set("saveCode",prepared["import_code"].as_str().unwrap())?;
                lua.load(r#"
                    local before=saveWrites
                    AO.ReviewMacroImport(saveCode); local receipt,ok=AO.ApplyReviewedMacro(saveCode)
                    assert(ok and receipt.status=='applied' and saveWrites==before+1)
                    for i=1,19 do assert(saveFlags[i]==false) end
                    AO.ReviewMacroImport(saveCode); assert(not pcall(AO.ApplyReviewedMacro,saveCode) and saveWrites==before+1)
                    AO.Capture()
                "#).exec()?;
                lua.globals().get("AzerothOperatorExport")
            })().map_err(|e|anyhow::anyhow!(e.to_string()))?;
            let verified = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
            anyhow::ensure!(
                verified.sections["operation_receipts"][prepared["operation_id"].as_str().unwrap()]
                    ["status"]
                    == "applied",
                "Packaged create/save receipt missing"
            );
        }
    }
    println!(
        "{}",
        serde_json::json!({"family":args[1],"variant":args[2],"interface":interface,
        "packaged_lua51_roundtrip":true,"changed_interface_rejected":true,"equipment_set_review_fixture":args[1]=="retail","equipment_set_use_fixture":args[1]=="retail","equipment_set_association_fixture":args[1]=="retail","equipment_set_save_fixture":args[1]=="retail","live_qualified":false})
    );
    Ok(())
}
