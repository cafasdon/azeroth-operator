use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionRequest {
    pub source_slot: u32,
    pub target_slot: u32,
}
fn hex(s: &str) -> String {
    if s.is_empty() {
        "-".into()
    } else {
        s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}
fn action(snapshot: &Snapshot, slot: u32) -> Result<(Value, Vec<String>)> {
    let mut entry = snapshot
        .sections
        .get("action_bar_observation")
        .and_then(|v| v["slots"].get(slot.to_string()))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Action slot not explicitly observed"))?;
    if entry["empty"] == true {
        return Ok((entry, vec!["-".into(); 6]));
    }
    ensure!(
        entry["sub_type"].as_str() != Some(""),
        "Empty action subtype is ambiguous"
    );
    let kind = entry["kind"].as_str().unwrap().to_owned();
    ensure!(
        matches!(kind.as_str(), "spell" | "item" | "macro"),
        "Unsupported action kind"
    );
    let id = entry["id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Action ID unavailable"))?;
    let mut fields = vec![
        kind.clone(),
        id.to_string(),
        hex(entry["sub_type"].as_str().unwrap_or("")),
        "-".into(),
        "-".into(),
        "-".into(),
    ];
    if kind == "macro" {
        let m = snapshot
            .sections
            .get("macros")
            .and_then(|v| v.get(id.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Macro preimage unavailable"))?;
        let name = m["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Macro name unavailable"))?;
        let body = m["body"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Macro body unavailable"))?;
        let icon = m["icon"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            .ok_or_else(|| anyhow::anyhow!("Numeric macro icon required"))?;
        entry["macro"] = json!({"name":name,"icon":icon,"body":body});
        fields[3] = hex(name);
        fields[4] = icon.to_string();
        fields[5] = hex(body);
    }
    Ok((entry, fields))
}
pub fn compile(snapshot: &Snapshot, snapshot_id: &str, request: &ActionRequest) -> Result<Value> {
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
        (1..=4096).contains(&request.source_slot)
            && (1..=4096).contains(&request.target_slot)
            && request.source_slot != request.target_slot,
        "Invalid action locations"
    );
    let (source, source_fields) = action(snapshot, request.source_slot)?;
    let (target, target_fields) = action(snapshot, request.target_slot)?;
    ensure!(source["empty"] == false, "Source is empty");
    let mut f = vec![
        "AO-ACTION-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        request.source_slot.to_string(),
        request.target_slot.to_string(),
    ];
    f.extend(source_fields);
    f.extend(target_fields);
    f[1] = format!("{:x}", Sha256::digest(f.join(":").as_bytes()));
    let code = f.join(":");
    ensure!(code.len() <= 8192, "Action request exceeds import limit");
    Ok(
        json!({"operation_id":f[1],"snapshot_id":snapshot_id,"kind":"move_action_slot",
        "source_slot":request.source_slot,"target_slot":request.target_slot,"source":source,"target":target,
        "import_code":code,"requires_player_apply":true,"game_execution":false,"live_qualified":false,
        "addon_execution_enabled":true,"requires_addon_version":"0.9.0-dev","coverage":"two_explicitly_observed_slots","mode":if target["empty"]==true {"move"} else {"swap"}}),
    )
}
