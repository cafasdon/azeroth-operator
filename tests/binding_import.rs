use azeroth_operator::{
    Snapshot,
    binding_operation::{BindingRequest, compile},
};
use mlua::Lua;
use serde_json::json;

#[test]
fn compiled_bindings_preserve_context_and_reject_unknown_keys() -> anyhow::Result<()> {
    let snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2026-11-03T00:00:00Z",
        "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Example","realm":"Test"},
        "sections":{"binding_state":{"binding_set":2,"coverage":"bound_keys_only"},"bindings":{"1":{"command":"ACTIONBUTTON1","key1":"CTRL-1","context":0},"2":{"command":"HOUSING","key1":"CTRL-1","context":1}}}}),
    )?;
    let lua = Lua::new();
    let ao = lua.create_table().unwrap();
    lua.load(include_str!("../addon/BindingImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))
        .unwrap();
    let decoder: mlua::Function = ao.get("DecodeBindingOperation").unwrap();
    for (context, expected) in [(0, "ACTIONBUTTON1"), (1, "HOUSING")] {
        let result = compile(
            &snapshot,
            &"a".repeat(64),
            &BindingRequest {
                key: "CTRL-1".into(),
                context,
                desired: String::new(),
                expect_unbound: false,
            },
        )?;
        let decoded: mlua::Table = decoder
            .call(result["import_code"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded.get::<String>("expected").unwrap(), expected);
        assert_eq!(decoded.get::<u32>("context").unwrap(), context);
        assert_eq!(decoded.get::<u32>("binding_set").unwrap(), 2);
        assert_eq!(decoded.get::<String>("desired").unwrap(), "");
        if context == 0 {
            lua.globals().set("AO", ao.clone()).unwrap();
            lua.globals()
                .set("code", result["import_code"].as_str().unwrap())
                .unwrap();
            lua.load(r#"
                AO.family='retail'; AO.variant='live'; AO.interface=120105; binding='ACTIONBUTTON1'; writes=0
                function GetBuildInfo() return '', '70688', '',120105 end
                function UnitName() return 'Example' end
                function GetRealmName() return 'Test' end
                function InCombatLockdown() return false end
                function GetCurrentBindingSet() return 2 end
                function GetBindingAction() return binding end
                function SetBinding(key,action,context) assert(key=='CTRL-1' and context==0); binding=action or ''; writes=writes+1; return true end
                function SaveBindings(set) assert(set==2) end
                C_KeyBindings={GetBindingContextForAction=function() return 0 end}
            "#).exec().unwrap();
            for source in [
                include_str!("../addon/BindingOperations.lua"),
                include_str!("../addon/MacroImport.lua"),
            ] {
                lua.load(source)
                    .call::<()>(("AzerothOperator", ao.clone()))
                    .unwrap();
            }
            lua.load(
                r#"
                assert(not pcall(AO.ApplyReviewedMacro,code))
                assert(AO.ReviewMacroImport(code).kind=='set_binding')
                local receipt,ok=AO.ApplyReviewedMacro(code)
                assert(ok and receipt.status=='applied' and binding=='' and writes==1)
                local receipts=AO.ReadMacroReceipts()
                assert(receipts[AO.DecodeBindingOperation(code).id].kind=='set_binding')
                assert(AzerothOperatorOperations:find(':applied:set_binding;',1,true))
                AO.ReviewMacroImport(code)
                assert(not pcall(AO.ApplyReviewedMacro,code) and writes==1)
            "#,
            )
            .exec()
            .unwrap();
        }
    }
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &BindingRequest {
                key: "CTRL-2".into(),
                context: 0,
                desired: "ACTIONBUTTON2".into(),
                expect_unbound: false,
            }
        )
        .is_err()
    );
    assert!(decoder.call::<mlua::Table>("AO-BIND-1:bad").is_err());
    let new_key = BindingRequest {
        key: "CTRL-2".into(),
        context: 0,
        desired: "ACTIONBUTTON2".into(),
        expect_unbound: true,
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &new_key)?;
    assert_eq!(prepared["expected"], "");
    assert_eq!(
        prepared["precondition_basis"],
        "explicit_unbound_precondition_not_observed"
    );
    let occupied = BindingRequest {
        key: "CTRL-1".into(),
        ..new_key
    };
    assert!(compile(&snapshot, &"a".repeat(64), &occupied).is_err());
    Ok(())
}
