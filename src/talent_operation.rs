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
    Create,
    Import,
}
#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalentRequest {
    pub target_config_id: Option<u32>,
    #[serde(default)]
    pub operation: TalentOperation,
    pub name: Option<String>,
    pub import_string: Option<String>,
    pub allow_empty_hash: Option<bool>,
    pub expected_new_shared_action_bars: Option<bool>,
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
    let importing = request.operation == TalentOperation::Import;
    if importing {
        let text = request
            .import_string
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Import text required"))?;
        ensure!(
            (26..=16384).contains(&text.len())
                && text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/'),
            "Invalid import text"
        );
        ensure!(
            request.allow_empty_hash.is_some(),
            "Explicit empty-hash acknowledgement required"
        );
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bits: Vec<u8> = text
            .bytes()
            .take(26)
            .flat_map(|b| {
                let value = alphabet.iter().position(|x| *x == b).unwrap() as u8;
                (0..6).map(move |i| (value >> i) & 1)
            })
            .collect();
        let value = |start: usize, width: usize| -> u64 {
            (0..width).map(|i| u64::from(bits[start + i]) << i).sum()
        };
        ensure!(
            Some(value(0, 8)) == active["serialization_version"].as_u64()
                && Some(value(8, 16)) == active["spec_id"].as_u64(),
            "Import version/spec mismatch"
        );
        ensure!(
            bits[24..152].iter().any(|b| *b != 0) || request.allow_empty_hash == Some(true),
            "Import hash omitted without acknowledgement"
        );
    } else {
        ensure!(
            request.import_string.is_none() && request.allow_empty_hash.is_none(),
            "Unexpected import fields"
        );
    }
    if request.operation == TalentOperation::Create || importing {
        ensure!(
            request.target_config_id.is_none(),
            "Create cannot target an existing config"
        );
        return metadata_draft(snapshot, snapshot_id, request, talents);
    }
    ensure!(
        request.expected_new_shared_action_bars.is_none(),
        "Unexpected new-loadout setting"
    );
    let target = request
        .target_config_id
        .ok_or_else(|| anyhow::anyhow!("Target config required"))?;
    ensure!(target > 0, "Invalid target config");
    let desired = catalog["entries"]
        .get(target.to_string())
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
        target.to_string(),
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
    let importing = request.operation == TalentOperation::Import;
    let creating = request.operation == TalentOperation::Create || importing;
    let target = request.target_config_id.unwrap_or(0);
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
    if creating {
        ensure!(entries.len() < 128, "Captured loadout catalogue full");
        ensure!(
            request.expected_new_shared_action_bars.is_some(),
            "Expected new shared-bars state required"
        );
    }
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
    if rename || creating {
        let name = request
            .name
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Operation requires a name"))?;
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name.trim() == name
                && !name.bytes().any(|b| b < 32 || b == 127 || b == b'|'),
            "Invalid loadout name"
        );
        for (id, entry) in entries {
            ensure!(
                (!creating && id == &target.to_string())
                    || !entry["name"]
                        .as_str()
                        .unwrap_or_default()
                        .eq_ignore_ascii_case(name),
                "Loadout name collision"
            );
        }
    } else {
        ensure!(request.name.is_none(), "Delete does not accept a name");
    }
    let mut expected = json!({"spec_id":active["spec_id"],"active_config_id":working,"serialization_version":active["serialization_version"],"active_import_string":active["import_string"],"entries":entries});
    if !catalog["selected_config_id"].is_null() {
        expected["selected_config_id"] = catalog["selected_config_id"].clone();
    }
    let mut draft = json!({"format":"talent_metadata_draft_v1","kind":if rename {"rename_talent_config"} else {"delete_talent_config"},"snapshot_id":sid,"family":"retail","variant":snapshot.client.variant,"build":snapshot.client.build,"interface":snapshot.client.interface,"character":snapshot.character.name,"realm":snapshot.character.realm,"target_config_id":request.target_config_id,"expected":expected,"requires_player_apply":true,"game_execution":false,"live_qualified":false,"execution_supported":true,"addon_execution_enabled":true,"requires_addon_version":"0.14.0-dev","import_code":null,"action_bar_contents_verified":false,"completion":"requires_full_catalogue_and_active_content_readback"});
    if creating {
        draft["format"] = json!("talent_create_draft_v1");
        draft["kind"] = json!("create_talent_config");
        draft.as_object_mut().unwrap().remove("target_config_id");
        draft["expected_new_shared_action_bars"] = json!(request.expected_new_shared_action_bars);
        draft["execution_supported"] = json!(true);
        draft["addon_execution_enabled"] = json!(true);
        draft["requires_addon_version"] = json!("0.15.0-dev");
        draft["completion"] =
            json!("requires_unique_new_selected_build_and_preserved_active_contents");
        draft["selects_new_build"] = json!(true);
    }
    if importing {
        draft["format"] = json!("talent_import_draft_v1");
        draft["kind"] = json!("import_talent_config");
        draft["import_string"] = json!(request.import_string);
        draft["allow_empty_hash"] = json!(request.allow_empty_hash);
        draft["changes_active_talents"] = json!(true);
        draft["execution_supported"] = json!(false);
        draft["addon_execution_enabled"] = json!(false);
        draft["requires_addon_version"] = Value::Null;
        draft["completion"] =
            json!("requires_unique_new_selected_build_requested_active_and_saved_contents");
    }
    if !rename && !creating && catalog["selected_config_id"].as_u64() == Some(target as u64) {
        draft["clears_saved_selection"] = json!(true);
        draft["execution_supported"] = json!(true);
        draft["addon_execution_enabled"] = json!(true);
        draft["requires_addon_version"] = json!("0.16.0-dev");
        draft["completion"] =
            json!("requires_removed_target_cleared_selection_and_preserved_active_contents");
    }
    if rename || creating {
        draft["name"] = json!(request.name);
    }
    let mut fields = vec![
        if importing {
            "AO-TALIMPORT-1".into()
        } else if creating {
            "AO-TALCREATE-1".into()
        } else {
            "AO-TALMETA-1".into()
        },
        String::new(),
        sid.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        draft["kind"].as_str().unwrap().into(),
        if creating {
            "-".into()
        } else {
            target.to_string()
        },
        hex(request.name.as_deref().unwrap_or_default()),
        active["spec_id"].to_string(),
        working.to_string(),
        catalog["selected_config_id"]
            .as_u64()
            .unwrap_or(0)
            .to_string(),
        active["serialization_version"].to_string(),
        hex(active["import_string"].as_str().unwrap()),
        entries.len().to_string(),
    ];
    if creating {
        fields.push(if request.expected_new_shared_action_bars == Some(true) {
            "1".into()
        } else {
            "0".into()
        });
    }
    if importing {
        fields.push(if request.allow_empty_hash == Some(true) {
            "1".into()
        } else {
            "0".into()
        });
        fields.push(hex(request.import_string.as_deref().unwrap()));
    }
    for (id, entry) in entries {
        fields.extend([
            id.clone(),
            hex(entry["name"].as_str().unwrap()),
            if entry["uses_shared_action_bars"] == true {
                "1".into()
            } else {
                "0".into()
            },
            entry["serialization_version"].to_string(),
            hex(entry["import_string"].as_str().unwrap()),
        ]);
    }
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    let code = fields.join(":");
    ensure!(
        code.len() <= 524288,
        "Talent metadata transport exceeds size limit"
    );
    draft["operation_id"] = json!(fields[1]);
    draft["import_code"] = json!(code);
    Ok(draft)
}
