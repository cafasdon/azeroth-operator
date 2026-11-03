use std::process::Command;

#[test]
fn actual_import_cli_and_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("export.json");
    let store_path = dir.path().join("store");
    std::fs::write(&input,r#"{"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"beta","build":"123","interface":16001},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-operator"))
        .env("AZEROTH_DATA_DIR", &store_path)
        .arg("import")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let id = String::from_utf8(output.stdout).unwrap();
    let store = azeroth_operator::Store { root: store_path };
    assert_eq!(
        store.read(id.trim()).unwrap().client.family,
        azeroth_operator::Family::Forever
    );
    assert_eq!(
        store.list(azeroth_operator::Family::Forever, 0, 1).unwrap()["snapshots"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store.list(azeroth_operator::Family::Retail, 0, 1).unwrap()["snapshots"],
        serde_json::json!([])
    );
    assert!(store.list(azeroth_operator::Family::Forever, 0, 0).is_err());
    std::fs::write(&input, "AzerothOperatorExport = os.execute('bad')").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_azeroth-operator"))
        .env("AZEROTH_DATA_DIR", &store.root)
        .arg("import")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
}
