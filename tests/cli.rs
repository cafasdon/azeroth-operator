use std::process::Command;
use std::{io::Write, process::Stdio};

#[test]
fn sync_missing_export_does_not_require_transport() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("receipts");
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-operator"))
        .env_remove("AZEROTH_UPLOAD_SSH_CONFIG")
        .args(["sync", "retail"])
        .arg(dir.path().join("missing.lua"))
        .arg(&state)
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "waiting_for_export");
    assert!(!state.exists());
}

fn stdin_import(root: &std::path::Path, input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_azeroth-operator"))
        .env("AZEROTH_DATA_DIR", root)
        .arg("import-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Oversized input may cause the bounded reader to close early.
    if let Err(error) = stdin.write_all(input) {
        assert!(matches!(error.kind(), std::io::ErrorKind::BrokenPipe));
    }
    drop(stdin);
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_import_retries_and_rejections() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("snapshots");
    let valid = br#"{"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}}"#;
    let first = stdin_import(&root, valid);
    assert!(first.status.success());
    let retry = stdin_import(&root, valid);
    assert!(retry.status.success());
    assert_eq!(first.stdout, retry.stdout);
    for invalid in [
        b"{}".as_slice(),
        b"AzerothOperatorExport = os.execute('bad')",
        b"",
    ] {
        let result = stdin_import(&root, invalid);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
    let oversized = vec![b' '; (azeroth_operator::MAX_BYTES * 2 + 257) as usize];
    assert!(!stdin_import(&root, &oversized).status.success());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

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
