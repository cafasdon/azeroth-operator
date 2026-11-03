//! Fixed SSH upload channel. Remote forced command and dedicated key are deployment prerequisites.
use crate::{MAX_BYTES, Snapshot};
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{process::Stdio, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

async fn exchange(
    mut input: impl AsyncWrite + Unpin,
    output: impl AsyncRead + Unpin,
    payload: &[u8],
    expected: &str,
) -> Result<()> {
    input.write_all(payload).await?;
    input.shutdown().await?;
    drop(input);
    let mut receipt = Vec::new();
    output.take(130).read_to_end(&mut receipt).await?;
    ensure!(
        receipt == format!("{expected}\n").as_bytes(),
        "Upload receipt mismatch; outcome unknown"
    );
    Ok(())
}

pub async fn upload(snapshot: &Snapshot) -> Result<String> {
    snapshot.validate()?;
    let config = std::env::var_os("AZEROTH_UPLOAD_SSH_CONFIG")
        .ok_or_else(|| anyhow::anyhow!("Dedicated upload SSH configuration is not configured"))?;
    ensure!(
        std::path::Path::new(&config).is_absolute(),
        "Upload SSH configuration must be absolute"
    );
    let payload = serde_json::to_vec(snapshot)?;
    ensure!(payload.len() as u64 <= MAX_BYTES, "Snapshot too large");
    let expected = format!("{:x}", Sha256::digest(&payload));
    let mut child = tokio::process::Command::new("ssh")
        .arg("-F")
        .arg(config)
        .args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "ConnectTimeout=15",
            "-o",
            "ServerAliveInterval=10",
            "-o",
            "ServerAliveCountMax=2",
            "azeroth-upload",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        exchange(input, output, &payload, &expected).await?;
        ensure!(
            child.wait().await?.success(),
            "Upload process failed; outcome unknown"
        );
        Ok::<(), anyhow::Error>(())
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(expected),
        failure => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            match failure {
                Ok(Err(error)) => Err(error),
                _ => anyhow::bail!(
                    "Upload timed out; outcome unknown. Retrying identical data is idempotent"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn receipt_and_payload_roundtrip() {
        for correct in [true, false] {
            let (client, mut peer) = tokio::io::duplex(64);
            let (reader, writer) = tokio::io::split(client);
            let server = tokio::spawn(async move {
                let mut input = Vec::new();
                peer.read_to_end(&mut input).await.unwrap();
                assert_eq!(input, b"payload");
                peer.write_all(if correct { b"receipt\n" } else { b"wrong\n" })
                    .await
                    .unwrap();
            });
            assert_eq!(
                exchange(writer, reader, b"payload", "receipt")
                    .await
                    .is_ok(),
                correct
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn stalled_peer_is_cancellable() {
        let (client, _peer) = tokio::io::duplex(64);
        let (reader, writer) = tokio::io::split(client);
        assert!(
            tokio::time::timeout(
                Duration::from_millis(20),
                exchange(writer, reader, b"payload", "receipt")
            )
            .await
            .is_err()
        );
    }
}
