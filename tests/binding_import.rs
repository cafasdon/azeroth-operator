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
            },
        )?;
        let decoded: mlua::Table = decoder
            .call(result["import_code"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded.get::<String>("expected").unwrap(), expected);
        assert_eq!(decoded.get::<u32>("context").unwrap(), context);
        assert_eq!(decoded.get::<u32>("binding_set").unwrap(), 2);
        assert_eq!(decoded.get::<String>("desired").unwrap(), "");
    }
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &BindingRequest {
                key: "CTRL-2".into(),
                context: 0,
                desired: "ACTIONBUTTON2".into()
            }
        )
        .is_err()
    );
    assert!(decoder.call::<mlua::Table>("AO-BIND-1:bad").is_err());
    Ok(())
}
