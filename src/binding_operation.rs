//! Compile one observed binding change for explicit player application.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingRequest {
    pub key: String,
    pub context: u32,
    pub desired: String,
}
fn hex(value: &str) -> String {
    if value.is_empty() {
        "-".into()
    } else {
        value
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
pub fn compile(snapshot: &Snapshot, snapshot_id: &str, request: &BindingRequest) -> Result<Value> {
    snapshot.validate()?;
    ensure!(
        snapshot.client.family == Family::Retail,
        "Unsupported client"
    );
    ensure!(
        snapshot_id.len() == 64
            && snapshot_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Invalid snapshot ID"
    );
    ensure!(
        !request.key.is_empty()
            && request.key.len() <= 128
            && !request.key.chars().any(char::is_control),
        "Invalid key"
    );
    ensure!(
        request.desired.len() <= 256
            && !request.desired.chars().any(char::is_control)
            && request.context <= 9,
        "Invalid action/context"
    );
    let state = snapshot
        .sections
        .get("binding_state")
        .ok_or_else(|| anyhow::anyhow!("Binding set unavailable"))?;
    let set = state["binding_set"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Binding set unavailable"))?;
    let bindings = snapshot
        .sections
        .get("bindings")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Bindings unavailable"))?;
    let matches: Vec<_> = bindings
        .values()
        .filter(|b| {
            b["context"].as_u64() == Some(request.context.into())
                && (b["key1"].as_str() == Some(&request.key)
                    || b["key2"].as_str() == Some(&request.key))
        })
        .collect();
    ensure!(
        matches.len() == 1,
        "Key/context must have one captured binding; uncaptured keys are unknown"
    );
    let expected = matches[0]["command"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing action"))?;
    ensure!(
        expected.len() <= 256 && !expected.chars().any(char::is_control),
        "Invalid captured action"
    );
    let mut fields = vec![
        "AO-BIND-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        hex(&request.key),
        hex(expected),
        hex(&request.desired),
        set.to_string(),
        request.context.to_string(),
    ];
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    Ok(
        json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"kind":"set_binding","key":request.key,"context":request.context,"binding_set":set,"expected":expected,"desired":request.desired,"import_code":fields.join(":"),"requires_player_apply":true,"game_execution":false,"live_qualified":false}),
    )
}
