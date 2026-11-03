use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRequest {
    pub bag: u32,
    pub slot: u32,
    pub target_slot: u32,
    #[serde(default)]
    pub expect_empty_target: bool,
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
pub fn compile(
    snapshot: &Snapshot,
    snapshot_id: &str,
    request: &EquipmentRequest,
) -> Result<Value> {
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
        request.bag <= 64
            && (1..=1024).contains(&request.slot)
            && (1..=19).contains(&request.target_slot),
        "Invalid locations"
    );
    let source = snapshot
        .sections
        .get("bags")
        .and_then(|b| b.get(request.bag.to_string()))
        .and_then(|b| b.get(request.slot.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Source not captured"))?;
    ensure!(
        source["count"].as_u64() == Some(1),
        "Equipment source must be a single item"
    );
    let guid = source["guid"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Source GUID unavailable"))?;
    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Equipment unavailable"))?;
    let target = equipment.get(&request.target_slot.to_string());
    let target_guid = if let Some(target) = target {
        ensure!(!request.expect_empty_target, "Target captured occupied");
        target["guid"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Target GUID unavailable"))?
    } else {
        ensure!(
            request.expect_empty_target,
            "Uncaptured target requires explicit empty precondition"
        );
        ""
    };
    ensure!(guid != target_guid, "Source and target identity collide");
    let item_id = source["item_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Source item ID unavailable"))?;
    let mut fields = vec![
        "AO-EQUIP-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        request.bag.to_string(),
        request.slot.to_string(),
        request.target_slot.to_string(),
        item_id.to_string(),
        hex(guid),
        hex(target_guid),
    ];
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    Ok(
        json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"kind":"equip_bag_item","bag":request.bag,"slot":request.slot,"target_slot":request.target_slot,"source":source,"target":target,"empty_target_is_explicit_precondition":request.expect_empty_target,"import_code":fields.join(":"),"requires_player_apply":true,"game_execution":false,"live_qualified":false,"compatibility":"checked_in_game","multi_slot_handling":"not_implemented"}),
    )
}
