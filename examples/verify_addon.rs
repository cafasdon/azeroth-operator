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
                set_id: 0,
                name: Some("Verified package".into()),
                icon: Some(123),
                expect_empty_slots: vec![],
                spec_index: None,
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
                set_id: 0,
                name: None,
                icon: None,
                expect_empty_slots: (1..=19).filter(|i| *i != 16).collect(),
                spec_index: None,
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
    }
    println!(
        "{}",
        serde_json::json!({"family":args[1],"variant":args[2],"interface":interface,
        "packaged_lua51_roundtrip":true,"changed_interface_rejected":true,"equipment_set_review_fixture":args[1]=="retail","equipment_set_use_fixture":args[1]=="retail","live_qualified":false})
    );
    Ok(())
}
