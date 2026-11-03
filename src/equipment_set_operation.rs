use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSetAction {
    Modify,
    Delete,
    Use,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSetRequest {
    pub operation: EquipmentSetAction,
    pub set_id: u32,
    pub name: Option<String>,
    pub icon: Option<u32>,
    #[serde(default)]
    pub expect_empty_slots: Vec<u32>,
}
fn hex(s: &str) -> String {
    if s.is_empty() {
        "-".into()
    } else {
        s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "-".into(),
        Value::Bool(v) => if *v { "1" } else { "0" }.into(),
        Value::String(s) => hex(s),
        _ => value.to_string(),
    }
}
pub fn compile(
    snapshot: &Snapshot,
    snapshot_id: &str,
    request: &EquipmentSetRequest,
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
    let catalogue = snapshot
        .sections
        .get("equipment_sets")
        .ok_or_else(|| anyhow::anyhow!("Equipment set catalogue unavailable"))?;
    ensure!(catalogue["can_use"] == true, "Equipment sets unavailable");
    let entries = catalogue["entries"].as_object().unwrap();
    let expected = entries
        .get(&request.set_id.to_string())
        .ok_or_else(|| anyhow::anyhow!("Equipment set not captured"))?;
    let is_use = matches!(request.operation, EquipmentSetAction::Use);
    ensure!(
        is_use || request.expect_empty_slots.is_empty(),
        "Empty-slot preconditions only apply to use"
    );
    let mut equipment_before = serde_json::Map::new();
    let mut desired_equipment = serde_json::Map::new();
    let kind = match request.operation {
        EquipmentSetAction::Modify => {
            let name = request
                .name
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Name required"))?;
            ensure!(
                !name.is_empty()
                    && name.len() <= 64
                    && name.trim() == name
                    && !name.chars().any(|c| c.is_control() || c == '|'),
                "Invalid set name"
            );
            ensure!(
                request.icon.is_some_and(|n| n > 0),
                "Positive icon required"
            );
            ensure!(
                entries
                    .iter()
                    .all(|(id, e)| id == &request.set_id.to_string()
                        || !e["name"].as_str().unwrap().eq_ignore_ascii_case(name)),
                "Equipment set name already exists"
            );
            "modify_equipment_set"
        }
        EquipmentSetAction::Delete => {
            ensure!(
                request.name.is_none() && request.icon.is_none(),
                "Delete must not contain replacement metadata"
            );
            "delete_equipment_set"
        }
        EquipmentSetAction::Use => {
            ensure!(
                request.name.is_none() && request.icon.is_none(),
                "Use must not contain replacement metadata"
            );
            ensure!(expected["num_lost"] == 0, "Equipment set has missing items");
            let equipment = snapshot
                .sections
                .get("equipment")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow::anyhow!("Equipment unavailable"))?;
            let empty: std::collections::BTreeSet<_> =
                request.expect_empty_slots.iter().copied().collect();
            ensure!(
                empty.len() == request.expect_empty_slots.len()
                    && empty.iter().all(|s| (1..=19).contains(s)),
                "Invalid empty-slot preconditions"
            );
            let mut identities = std::collections::BTreeSet::new();
            for slot in 1..=19 {
                let key = slot.to_string();
                let before = if let Some(item) = equipment.get(&key) {
                    ensure!(
                        !empty.contains(&slot),
                        "Empty precondition conflicts with captured equipment"
                    );
                    item["guid"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Equipment GUID unavailable"))?
                } else {
                    ensure!(
                        empty.contains(&slot),
                        "Uncaptured equipment requires explicit empty precondition"
                    );
                    ""
                };
                equipment_before.insert(key.clone(), json!(before));
                let target = &expected["slots"][&key];
                let desired = if target["ignored"] == true {
                    before
                } else {
                    ensure!(
                        target["ignored"] == false && target["observed_item"]["locked"] == false,
                        "Unresolved or locked equipment set slot"
                    );
                    target["observed_item"]["guid"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Target GUID unavailable"))?
                };
                ensure!(
                    desired.is_empty() || identities.insert(desired),
                    "Duplicate target identity"
                );
                desired_equipment.insert(key, json!(desired));
            }
            "use_equipment_set"
        }
    };
    let mut f = vec![
        if is_use { "AO-SET-USE-1" } else { "AO-SET-1" }.into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        kind.into(),
        request.set_id.to_string(),
        request
            .name
            .as_deref()
            .map(hex)
            .unwrap_or_else(|| "-".into()),
        request
            .icon
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into()),
    ];
    for key in [
        "name",
        "icon",
        "is_equipped",
        "num_items",
        "num_equipped",
        "num_in_inventory",
        "num_lost",
        "num_ignored",
        "assigned_spec",
    ] {
        f.push(scalar(&expected[key]));
    }
    for slot in 1..=19 {
        let s = &expected["slots"][slot.to_string()];
        for key in ["ignored", "raw_item_id", "raw_location"] {
            f.push(scalar(&s[key]));
        }
        let observed = &s["observed_item"];
        for key in ["kind", "slot", "bag", "item_id", "guid", "locked"] {
            f.push(scalar(&observed[key]));
        }
    }
    if is_use {
        for slot in 1..=19 {
            f.push(scalar(&equipment_before[&slot.to_string()]));
        }
    }
    f[1] = format!("{:x}", Sha256::digest(f.join(":").as_bytes()));
    let code = f.join(":");
    ensure!(
        code.len() <= 8192,
        "Equipment set request exceeds import limit"
    );
    Ok(
        json!({"operation_id":f[1],"snapshot_id":snapshot_id,"kind":kind,"set_id":request.set_id,
        "expected":expected,"name":request.name,"icon":request.icon,"import_code":code,
        "requires_player_apply":true,"game_execution":false,"addon_execution_enabled":!is_use,"requires_addon_version":if is_use { None } else { Some("0.10.0-dev") },"live_qualified":false,
        "equipment_before":if is_use { Some(&equipment_before) } else { None },
        "desired_equipment":if is_use { Some(&desired_equipment) } else { None },
        "explicit_empty_preconditions":request.expect_empty_slots,
        "deletes_saved_set":matches!(request.operation,EquipmentSetAction::Delete),
        "assigned_spec":expected.get("assigned_spec"),"external_references_repaired":false}),
    )
}
