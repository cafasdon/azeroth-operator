use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[derive(Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TalentOperation {
    #[default]
    Load,
    Rename,
    Delete,
}
#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalentRequest {
    pub target_config_id: u32,
    #[serde(default)]
    pub operation: TalentOperation,
    pub name: Option<String>,
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
    if request.operation != TalentOperation::Load {
        return metadata_draft(snapshot, snapshot_id, request, talents);
    }
    ensure!(request.name.is_none(), "Load does not accept a name");
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

fn metadata_draft(
    snapshot: &Snapshot,
    sid: &str,
    request: &TalentRequest,
    talents: &Value,
) -> Result<Value> {
    let active = &talents["loadout_export"];
    let catalog = &talents["saved_loadouts"];
    let entries = catalog["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue"))?;
    let working = talents["config_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing working config"))?;
    ensure!(
        working > 0 && working <= u32::MAX as u64,
        "Invalid working config"
    );
    ensure!(
        entries.len() <= 128 && !entries.contains_key(&working.to_string()),
        "Ambiguous catalogue"
    );
    for entry in entries.values() {
        ensure!(
            entry["serialization_version"] == active["serialization_version"],
            "Catalogue serialization version mismatch"
        );
        let name = entry["name"].as_str().unwrap();
        ensure!(
            !name.is_empty()
                && name.len() <= 1024
                && entry["import_string"].as_str().unwrap().len() <= 16384,
            "Catalogue entry exceeds executor limits"
        );
    }
    ensure!(
        active["import_string"].as_str().unwrap().len() <= 16384,
        "Active import exceeds executor limit"
    );
    let rename = request.operation == TalentOperation::Rename;
    if rename {
        let name = request
            .name
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Rename requires a name"))?;
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name.trim() == name
                && !name.bytes().any(|b| b < 32 || b == 127 || b == b'|'),
            "Invalid loadout name"
        );
        for (id, entry) in entries {
            ensure!(
                id == &request.target_config_id.to_string()
                    || !entry["name"]
                        .as_str()
                        .unwrap_or_default()
                        .eq_ignore_ascii_case(name),
                "Loadout name collision"
            );
        }
    } else {
        ensure!(request.name.is_none(), "Delete does not accept a name");
        ensure!(
            catalog["selected_config_id"].as_u64() != Some(request.target_config_id as u64),
            "Deleting the selected saved loadout is not supported"
        );
    }
    let mut expected = json!({"spec_id":active["spec_id"],"active_config_id":working,"serialization_version":active["serialization_version"],"active_import_string":active["import_string"],"entries":entries});
    if !catalog["selected_config_id"].is_null() {
        expected["selected_config_id"] = catalog["selected_config_id"].clone();
    }
    let mut draft = json!({"format":"talent_metadata_draft_v1","kind":if rename {"rename_talent_config"} else {"delete_talent_config"},"snapshot_id":sid,"family":"retail","variant":snapshot.client.variant,"build":snapshot.client.build,"interface":snapshot.client.interface,"character":snapshot.character.name,"realm":snapshot.character.realm,"target_config_id":request.target_config_id,"expected":expected,"requires_player_apply":true,"game_execution":false,"live_qualified":false,"execution_supported":false,"import_code":null,"action_bar_contents_verified":false,"completion":"requires_full_catalogue_and_active_content_readback"});
    if rename {
        draft["name"] = json!(request.name);
    }
    let bytes = serde_json::to_vec(&draft)?;
    ensure!(
        bytes.len() <= 524288,
        "Talent metadata draft exceeds size limit"
    );
    draft["operation_id"] = json!(format!("{:x}", Sha256::digest(bytes)));
    Ok(draft)
}
