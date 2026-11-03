use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalentRequest {
    pub target_config_id: u32,
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
pub fn compile(snapshot: &Snapshot, snapshot_id: &str, request: &TalentRequest) -> Result<Value> {
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
        "Invalid snapshot identity"
    );
    ensure!(request.target_config_id > 0, "Invalid target config");
    let talents = snapshot
        .sections
        .get("talents")
        .ok_or_else(|| anyhow::anyhow!("Talents unavailable"))?;
    let active = &talents["loadout_export"];
    ensure!(
        active["status"] == "available" && active["staged_changes"] == false,
        "Committed talent contents unavailable"
    );
    let catalog = &talents["saved_loadouts"];
    ensure!(
        catalog["spec_id"] == active["spec_id"],
        "Loadout spec mismatch"
    );
    let desired = catalog["entries"]
        .get(request.target_config_id.to_string())
        .ok_or_else(|| anyhow::anyhow!("Saved loadout not captured"))?;
    ensure!(
        desired["serialization_version"] == active["serialization_version"],
        "Serialization version mismatch"
    );
    let text = |v: &Value| -> Result<String> {
        Ok(hex(v
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing text"))?))
    };
    let mut fields = vec![
        "AO-TALENT-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        catalog["spec_id"].to_string(),
        talents["config_id"].to_string(),
        request.target_config_id.to_string(),
        active["serialization_version"].to_string(),
        text(&active["import_string"])?,
        text(&desired["import_string"])?,
        text(&desired["name"])?,
        if desired["uses_shared_action_bars"] == true {
            "1".into()
        } else {
            "0".into()
        },
        catalog["selected_config_id"]
            .as_u64()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "0".into()),
    ];
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    let code = fields.join(":");
    ensure!(
        code.len() <= 8192,
        "Talent operation exceeds review transport limit"
    );
    Ok(
        json!({"kind":"load_talent_config","operation_id":fields[1],"snapshot_id":snapshot_id,"target_config_id":request.target_config_id,"desired":desired,"expected_committed":active,"expected_active_config_id":talents["config_id"],"expected_selected_config_id":catalog["selected_config_id"],"import_code":code,"requires_player_apply":true,"game_execution":false,"live_qualified":false,"completion":"requires_committed_content_readback","action_bar_contents_verified":false}),
    )
}
