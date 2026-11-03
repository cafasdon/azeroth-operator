//! Durable upload acknowledgements, not a live remote-state cache.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    io::{Read, Write},
    path::Path,
};

pub async fn sync_file(file: &Path, family: Family, state: &Path) -> Result<Value> {
    let snapshot = match crate::import::import_file(file) {
        Ok(s) => s,
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(json!({"status":"waiting_for_export","live":false}));
        }
        Err(e) => return Err(e),
    };
    ensure!(snapshot.client.family == family, "Export family mismatch");
    let config = std::env::var_os("AZEROTH_UPLOAD_SSH_CONFIG")
        .ok_or_else(|| anyhow::anyhow!("Upload configuration missing"))?;
    ensure!(
        Path::new(&config).is_absolute(),
        "Upload configuration must be absolute"
    );
    let mut destination = Vec::new();
    std::fs::File::open(config)?
        .take(65537)
        .read_to_end(&mut destination)?;
    ensure!(
        !destination.is_empty() && destination.len() <= 65536,
        "Invalid configuration size"
    );
    sync_with(&snapshot, &destination, state, || {
        crate::upload::upload(&snapshot)
    })
    .await
}

async fn sync_with<F, Fut>(
    snapshot: &Snapshot,
    destination: &[u8],
    state: &Path,
    send: F,
) -> Result<Value>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String>>,
{
    snapshot.validate()?;
    let id = format!("{:x}", Sha256::digest(serde_json::to_vec(snapshot)?));
    let binding = format!("{:x}", Sha256::digest(destination));
    let marker = format!("{id}\n{binding}\n");
    let key = format!("{:x}", Sha256::digest(marker.as_bytes()));
    let path = state.join(format!("{key}.receipt"));
    if let Ok(file) = std::fs::File::open(&path) {
        let mut bytes = Vec::new();
        file.take(256).read_to_end(&mut bytes)?;
        if bytes == marker.as_bytes() {
            return Ok(
                json!({"status":"unchanged","snapshot_id":id,"previously_acknowledged":true,"remote_rechecked":false}),
            );
        }
    }
    ensure!(
        send().await? == id,
        "Receipt mismatch; no acknowledgement saved"
    );
    std::fs::create_dir_all(state)?;
    let mut temporary = tempfile::NamedTempFile::new_in(state)?;
    temporary.write_all(marker.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(json!({"status":"uploaded","snapshot_id":id,"remote_acknowledged":true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[tokio::test]
    async fn retry_failures_skip_acked_and_resend_for_new_destination() {
        let s: Snapshot = serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}})).unwrap();
        let id = format!("{:x}", Sha256::digest(serde_json::to_vec(&s).unwrap()));
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("receipts");
        assert!(
            sync_with(&s, b"a", &state, || async { anyhow::bail!("offline") })
                .await
                .is_err()
        );
        assert!(!state.exists());
        assert!(
            sync_with(&s, b"a", &state, || async { Ok("wrong".into()) })
                .await
                .is_err()
        );
        assert!(!state.exists());
        let sends = Cell::new(0);
        for destination in [b"a", b"a", b"b"] {
            sync_with(&s, destination, &state, || async {
                sends.set(sends.get() + 1);
                Ok(id.clone())
            })
            .await
            .unwrap();
        }
        assert_eq!(sends.get(), 2);
    }
}
