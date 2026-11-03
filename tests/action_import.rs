use azeroth_operator::{
    Snapshot,
    action_operation::{ActionRequest, compile},
};
use mlua::Lua;
use serde_json::json;
#[test]
fn compiler_decoder_bind_slots_and_macro_contents() -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot: Snapshot = serde_json::from_value(
        json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
 "client":{"family":"retail","variant":"live","build":"70688","interface":120105},"character":{"name":"Test","realm":"Test"},
 "sections":{"action_bar_observation":{"coverage":"loaded_builtin_buttons_only","observed_buttons":3,"missing_buttons":0,
 "slots":{"1":{"empty":false,"kind":"spell","id":123},"2":{"empty":false,"kind":"macro","id":121},"3":{"empty":true}}},
 "macros":{"121":{"name":"Macro","icon":134400,"body":"/cast Test\n/say test: value | literal","scope":"character"}}}}),
    )?;
    let request = ActionRequest {
        source_slot: 1,
        target_slot: 2,
    };
    let prepared = compile(&snapshot, &"a".repeat(64), &request)?;
    let lua = Lua::new();
    let ao = lua.create_table()?;
    lua.load(include_str!("../addon/ActionImport.lua"))
        .call::<()>(("AzerothOperator", ao.clone()))?;
    let decoder: mlua::Function = ao.get("DecodeActionOperation")?;
    let code = prepared["import_code"].as_str().unwrap();
    let op: mlua::Table = decoder.call(code)?;
    assert_eq!(op.get::<u32>("source_slot")?, 1);
    let target: mlua::Table = op.get("target")?;
    let m: mlua::Table = target.get("macro")?;
    assert_eq!(
        m.get::<String>("body")?,
        "/cast Test\n/say test: value | literal"
    );
    assert_eq!(prepared["mode"], "swap");
    let moved = compile(
        &snapshot,
        &"a".repeat(64),
        &ActionRequest {
            source_slot: 1,
            target_slot: 3,
        },
    )?;
    assert_eq!(moved["mode"], "move");
    let op: mlua::Table = decoder.call(moved["import_code"].as_str().unwrap())?;
    assert!(op.get::<mlua::Table>("target")?.get::<bool>("empty")?);
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                source_slot: 1,
                target_slot: 4
            }
        )
        .is_err()
    );
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                source_slot: 3,
                target_slot: 1
            }
        )
        .is_err()
    );
    assert!(
        compile(
            &snapshot,
            &"a".repeat(64),
            &ActionRequest {
                source_slot: 1,
                target_slot: 1
            }
        )
        .is_err()
    );
    for (field, bad) in [
        (9, "01"),
        (10, "4097"),
        (11, "flyout"),
        (18, "0"),
        (20, "zz"),
    ] {
        let mut f: Vec<_> = code.split(':').collect();
        f[field] = bad;
        assert!(decoder.call::<mlua::Table>(f.join(":")).is_err());
    }
    snapshot.sections.get_mut("macros").unwrap()["121"]["body"] = json!("changed");
    assert_ne!(
        compile(&snapshot, &"a".repeat(64), &request)?["operation_id"],
        prepared["operation_id"]
    );
    snapshot.sections.get_mut("macros").unwrap()["121"]["body"] = json!("x".repeat(5000));
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    snapshot.sections.remove("macros");
    assert!(compile(&snapshot, &"a".repeat(64), &request).is_err());
    Ok(())
}
