use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BagRequest {
    pub bag: u32,
    pub slot: u32,
    pub target_bag: u32,
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
pub fn compile(snapshot: &Snapshot, snapshot_id: &str, request: &BagRequest) -> Result<Value> {
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
            && request.target_bag <= 64
            && (1..=1024).contains(&request.target_slot),
        "Invalid locations"
    );
    let source = snapshot
        .sections
        .get("bags")
        .and_then(|b| b.get(request.bag.to_string()))
        .and_then(|b| b.get(request.slot.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Source not captured"))?;
    ensure!(
        request.bag != request.target_bag || request.slot != request.target_slot,
        "Source equals destination"
    );
    let count = source["count"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Source count unavailable"))?;
    let guid = source["guid"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Source GUID unavailable"))?;
    ensure!(
        request.expect_empty_target,
        "Explicit empty destination precondition required"
    );
    let target = snapshot
        .sections
        .get("bags")
        .and_then(|b| b.get(request.target_bag.to_string()))
        .and_then(|b| b.get(request.target_slot.to_string()));
    ensure!(target.is_none(), "Destination captured occupied");
    let item_id = source["item_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Source item ID unavailable"))?;
    let mut fields = vec![
        "AO-BAG-1".into(),
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
        request.target_bag.to_string(),
        request.target_slot.to_string(),
        item_id.to_string(),
        hex(guid),
        count.to_string(),
    ];
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    Ok(
        json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"kind":"move_bag_item",
        "source":source,"bag":request.bag,"slot":request.slot,"target_bag":request.target_bag,
        "target_slot":request.target_slot,"empty_target_is_explicit_precondition":true,
        "import_code":fields.join(":"),"requires_player_apply":true,"game_execution":false,
        "live_qualified":false,"destination_compatibility":"checked_in_game"}),
    )
}
