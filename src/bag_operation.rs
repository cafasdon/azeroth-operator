use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BagOperation {
    #[default]
    Move,
    Swap,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BagRequest {
    #[serde(default)]
    pub operation: BagOperation,
    /// Omit for a full-stack move; provide a smaller positive amount to split.
    pub amount: Option<u32>,
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
    if request.operation == BagOperation::Swap {
        ensure!(
            request.amount.is_none() && !request.expect_empty_target,
            "Swap cannot split or expect an empty target"
        );
        let target = snapshot
            .sections
            .get("bags")
            .and_then(|b| b.get(request.target_bag.to_string()))
            .and_then(|b| b.get(request.target_slot.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Swap target not captured"))?;
        for value in [
            &snapshot.client.variant,
            &snapshot.client.build,
            &snapshot.character.name,
            &snapshot.character.realm,
        ] {
            ensure!(
                !value.bytes().any(|b| b.is_ascii_control()),
                "Control character in swap identity"
            );
        }
        let mut fields = vec![
            "AO-BAGSWAP-1".into(),
            String::new(),
            snapshot_id.into(),
            "retail".into(),
            hex(&snapshot.client.variant),
            hex(&snapshot.client.build),
            snapshot.client.interface.to_string(),
            hex(&snapshot.character.name),
            hex(&snapshot.character.realm),
        ];
        let mut entries = Vec::new();
        for (entry, bag, slot) in [
            (source, request.bag, request.slot),
            (target, request.target_bag, request.target_slot),
        ] {
            let id = entry["item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Item ID missing"))?;
            let count = entry["count"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Count missing"))?;
            let guid = entry["guid"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow::anyhow!("GUID missing"))?;
            let link = entry["link"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow::anyhow!("Item link missing"))?;
            ensure!(
                !link.bytes().any(|b| b.is_ascii_control()),
                "Control character in item link"
            );
            fields.extend([
                bag.to_string(),
                slot.to_string(),
                id.to_string(),
                count.to_string(),
                hex(guid),
                hex(link),
            ]);
            entries.push(
                json!({"bag":bag,"slot":slot,"item_id":id,"count":count,"guid":guid,"link":link}),
            );
        }
        ensure!(
            entries[0]["guid"] != entries[1]["guid"]
                && entries[0]["item_id"] != entries[1]["item_id"],
            "Swap identities must differ; same-item stack compatibility unsupported"
        );
        fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
        let code = fields.join(":");
        ensure!(code.len() <= 8192, "Swap exceeds import limit");
        return Ok(
            json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"kind":"swap_bag_items",
            "source":entries[0],"target":entries[1],"import_code":code,"requires_player_apply":true,
            "game_execution":false,"live_qualified":false,"addon_execution_enabled":true,"requires_addon_version":"0.20.0-dev"}),
        );
    }
    let count = source["count"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Source count unavailable"))?;
    if let Some(amount) = request.amount {
        ensure!(
            amount > 0 && u64::from(amount) < count,
            "Split must leave a source remainder"
        );
    }
    let kind = if request.amount.is_some() {
        "split_bag_item"
    } else {
        "move_bag_item"
    };
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
    if let Some(amount) = request.amount {
        fields[0] = "AO-SPLIT-1".into();
        fields.push(amount.to_string());
    }
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    Ok(
        json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"kind":kind,"amount":request.amount,
        "source":source,"bag":request.bag,"slot":request.slot,"target_bag":request.target_bag,
        "target_slot":request.target_slot,"empty_target_is_explicit_precondition":true,
        "import_code":fields.join(":"),"requires_player_apply":true,"game_execution":false,
        "live_qualified":false,"addon_execution_enabled":true,"requires_addon_version":"0.20.0-dev","destination_compatibility":"metadata_preflight_and_native_readback"}),
    )
}
