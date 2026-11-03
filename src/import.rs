//! Parse our narrow SavedVariables wire format as data, never Lua code.
use crate::{MAX_BYTES, Snapshot};
use anyhow::{Result, ensure};
use std::{fs::File, io::Read, path::Path};
const WIRE_LIMIT: u64 = MAX_BYTES * 2 + 32768;

fn export_assignment(text: &str) -> Result<&str> {
    let mut remaining = text;
    let mut export = None;
    let mut receipts_seen = false;
    while !remaining.trim().is_empty() {
        let (name, rest) = remaining
            .trim_start()
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Expected assignment"))?;
        if name.trim() == "AzerothOperatorOperations"
            && let Some(tail) = rest.trim_start().strip_prefix("nil")
        {
            ensure!(
                !receipts_seen && (tail.is_empty() || tail.starts_with(char::is_whitespace)),
                "Invalid empty receipt checkpoint"
            );
            receipts_seen = true;
            remaining = tail;
            continue;
        }
        let rest = rest
            .trim_start()
            .strip_prefix('"')
            .ok_or_else(|| anyhow::anyhow!("Expected quoted data"))?;
        let (value, rest) = rest
            .split_once('"')
            .ok_or_else(|| anyhow::anyhow!("Unterminated data"))?;
        match name.trim() {
            "AzerothOperatorExport" => {
                ensure!(export.is_none(), "Duplicate export");
                export = Some(value);
            }
            "AzerothOperatorOperations" => {
                ensure!(
                    !receipts_seen && value.len() <= 16384,
                    "Invalid receipt checkpoint"
                );
                receipts_seen = true;
                let mut ids = std::collections::HashSet::new();
                if !value.is_empty() {
                    ensure!(value.ends_with(';'), "Invalid receipt terminator");
                    for row in value[..value.len() - 1].split(';') {
                        let fields: Vec<_> = row.split(':').collect();
                        ensure!(fields.len() == 3, "Invalid receipt row");
                        ensure!(
                            fields[0].len() == 64
                                && fields[0]
                                    .bytes()
                                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                                && ids.insert(fields[0]),
                            "Invalid receipt identity"
                        );
                        ensure!(
                            matches!(
                                fields[1],
                                "applied"
                                    | "outcome_unknown"
                                    | "copied_source_removal_unconfirmed"
                                    | "already_selected"
                            ) && matches!(
                                fields[2],
                                "create"
                                    | "edit"
                                    | "delete"
                                    | "move"
                                    | "set_binding"
                                    | "equip_bag_item"
                                    | "move_bag_item"
                                    | "split_bag_item"
                                    | "load_talent_config"
                                    | "move_action_slot"
                                    | "modify_equipment_set"
                                    | "delete_equipment_set"
                                    | "use_equipment_set"
                                    | "assign_equipment_set_spec"
                                    | "unassign_equipment_set_spec"
                            ),
                            "Invalid receipt result"
                        );
                        ensure!(ids.len() <= 128, "Too many receipts");
                    }
                }
            }
            _ => anyhow::bail!("Unknown assignment"),
        }
        remaining = rest;
    }
    export.ok_or_else(|| anyhow::anyhow!("Missing export"))
}

pub fn import_file(path: &Path) -> Result<Snapshot> {
    import_reader(File::open(path)?)
}

pub fn import_reader(reader: impl Read) -> Result<Snapshot> {
    let mut raw = Vec::new();
    reader.take(WIRE_LIMIT + 1).read_to_end(&mut raw)?;
    decode(&raw)
}

pub fn decode(raw: &[u8]) -> Result<Snapshot> {
    ensure!(raw.len() as u64 <= WIRE_LIMIT, "Export exceeds size limit");
    let text = std::str::from_utf8(raw)?
        .trim_start_matches('\u{feff}')
        .trim();
    let json = if text.starts_with('{') {
        text.as_bytes().to_vec()
    } else {
        let hex = export_assignment(text)?;
        ensure!(
            hex.len() % 2 == 0 && hex.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid hex export"
        );
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16))
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    ensure!(
        json.len() as u64 <= MAX_BYTES,
        "Snapshot exceeds size limit"
    );
    let snapshot: Snapshot = serde_json::from_slice(&json)?;
    snapshot.validate()?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = r#"{"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}}"#;
    #[test]
    fn json_and_saved_variables_agree() {
        let hex: String = FIXTURE.bytes().map(|b| format!("{b:02x}")).collect();
        let lua = format!("\u{feff}\nAzerothOperatorExport = \"{hex}\"\r\n");
        assert_eq!(
            decode(FIXTURE.as_bytes()).unwrap().character.name,
            decode(lua.as_bytes()).unwrap().character.name
        );
        assert!(decode(format!("{lua}; print('injected')").as_bytes()).is_err());
        assert!(decode(b"AzerothOperatorExport = os.execute('bad')").is_err());
        assert!(decode(b"AzerothOperatorExport = \"abc\"").is_err());
        assert!(decode(format!("{lua}\nAzerothOperatorOperations = nil\n").as_bytes()).is_ok());
        assert!(
            decode(format!("{lua}\nAzerothOperatorOperations = nilpotent\n").as_bytes()).is_err()
        );
        assert!(
            decode(
                format!("{lua}\nAzerothOperatorOperations = nil\nAzerothOperatorOperations = nil")
                    .as_bytes()
            )
            .is_err()
        );
        let receipt = format!("{}:applied:create;", "a".repeat(64));
        for combined in [
            format!("{lua}\nAzerothOperatorOperations = \"{receipt}\"\n"),
            format!("AzerothOperatorOperations = \"{receipt}\"\n{lua}"),
        ] {
            // A BOM belongs at the beginning of the file, never between assignments.
            let combined = combined.replace('\u{feff}', "");
            assert_eq!(
                decode(combined.as_bytes()).unwrap().character.name,
                "Example"
            );
        }
        for suffix in [
            "AzerothOperatorOperations = {}".to_string(),
            "AzerothOperatorOperations = os.execute('bad')".to_string(),
            format!("AzerothOperatorOperations = \"{receipt}{receipt}\""),
            format!("AzerothOperatorOperations = \"{receipt}\"\nprint('bad')"),
            "AzerothOperatorOperations = \"unknown\"".to_string(),
            "AzerothOperatorOperations = \"\"\nAzerothOperatorOperations = \"\"".to_string(),
        ] {
            assert!(decode(format!("{lua}{suffix}").as_bytes()).is_err());
        }
    }
}
