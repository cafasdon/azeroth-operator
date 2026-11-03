//! Parse our narrow SavedVariables wire format as data, never Lua code.
use crate::{MAX_BYTES, Snapshot};
use anyhow::{Result, ensure};
use std::{fs::File, io::Read, path::Path};

pub fn import_file(path: &Path) -> Result<Snapshot> {
    import_reader(File::open(path)?)
}

pub fn import_reader(reader: impl Read) -> Result<Snapshot> {
    let mut raw = Vec::new();
    reader.take(MAX_BYTES * 2 + 256).read_to_end(&mut raw)?;
    decode(&raw)
}

pub fn decode(raw: &[u8]) -> Result<Snapshot> {
    ensure!(
        raw.len() as u64 <= MAX_BYTES * 2 + 128,
        "Export exceeds size limit"
    );
    let text = std::str::from_utf8(raw)?
        .trim_start_matches('\u{feff}')
        .trim();
    let json = if text.starts_with('{') {
        text.as_bytes().to_vec()
    } else {
        let rest = text
            .strip_prefix("AzerothOperatorExport")
            .ok_or_else(|| anyhow::anyhow!("Unknown export format"))?
            .trim_start();
        let rest = rest
            .strip_prefix('=')
            .ok_or_else(|| anyhow::anyhow!("Expected assignment"))?
            .trim();
        let hex = rest
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or_else(|| anyhow::anyhow!("Expected quoted hex payload"))?;
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
    }
}
