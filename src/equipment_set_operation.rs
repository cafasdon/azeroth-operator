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
    AssignSpec,
    UnassignSpec,
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
    pub spec_index: Option<u32>,
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
    if matches!(
        request.operation,
        EquipmentSetAction::AssignSpec | EquipmentSetAction::UnassignSpec
    ) {
        return association_draft(snapshot, snapshot_id, request, catalogue);
    }
    ensure!(
        request.spec_index.is_none(),
        "Unexpected specialisation target"
    );
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
        EquipmentSetAction::AssignSpec | EquipmentSetAction::UnassignSpec => unreachable!(),
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
    f.extend(entry_fields(expected));
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
        "requires_player_apply":true,"game_execution":false,"addon_execution_enabled":true,"requires_addon_version":if is_use { "0.11.0-dev" } else { "0.10.0-dev" },"live_qualified":false,
        "equipment_before":if is_use { Some(&equipment_before) } else { None },
        "desired_equipment":if is_use { Some(&desired_equipment) } else { None },
        "explicit_empty_preconditions":request.expect_empty_slots,
        "deletes_saved_set":matches!(request.operation,EquipmentSetAction::Delete),
        "assigned_spec":expected.get("assigned_spec"),"external_references_repaired":false}),
    )
}

fn association_draft(
    snapshot: &Snapshot,
    snapshot_id: &str,
    request: &EquipmentSetRequest,
    catalogue: &Value,
) -> Result<Value> {
    ensure!(
        request.name.is_none() && request.icon.is_none() && request.expect_empty_slots.is_empty(),
        "Association must not contain gear or metadata replacements"
    );
    let specs = catalogue
        .get("specializations")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Specialisation identities unavailable"))?;
    let entries = catalogue["entries"].as_object().unwrap();
    let mut owners = std::collections::BTreeMap::new();
    for (id, entry) in entries {
        if let Some(spec) = entry.get("assigned_spec").and_then(Value::as_u64) {
            ensure!(
                owners.insert(spec, id).is_none(),
                "Ambiguous specialisation associations"
            );
        }
    }
    let assigning = matches!(request.operation, EquipmentSetAction::AssignSpec);
    let spec_id = if assigning {
        let index = request
            .spec_index
            .ok_or_else(|| anyhow::anyhow!("Specialisation index required"))?;
        Some(
            specs
                .get(&index.to_string())
                .ok_or_else(|| anyhow::anyhow!("Specialisation not captured"))?
                .clone(),
        )
    } else {
        ensure!(
            request.spec_index.is_none(),
            "Unassign must not contain target specialisation"
        );
        None
    };
    let key = request.set_id.to_string();
    let displaced = request
        .spec_index
        .and_then(|spec| owners.get(&(spec as u64)).copied())
        .filter(|id| **id != key);
    let mut desired = entries.clone();
    if let Some(id) = displaced {
        desired
            .get_mut(id)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("assigned_spec");
    }
    let target = desired.get_mut(&key).unwrap().as_object_mut().unwrap();
    if let Some(index) = request.spec_index {
        target.insert("assigned_spec".into(), json!(index));
    } else {
        target.remove("assigned_spec");
    }
    let mut draft = json!({"format":"equipment_set_association_draft_v1","snapshot_id":snapshot_id,
        "kind":if assigning {"assign_equipment_set_spec"} else {"unassign_equipment_set_spec"},
        "client":snapshot.client,"character":snapshot.character,"set_id":request.set_id,
        "spec_index":request.spec_index,"spec_id":spec_id,"expected_specializations":specs,
        "expected_catalogue":entries,"desired_catalogue":desired,
        "displaced_set":displaced.map(|id|json!({"set_id":id.parse::<u32>().unwrap(),"name":entries[id]["name"],"previous_spec":entries[id]["assigned_spec"]})),
        "future_specialisation_equipment_selection_may_change":true,
        "already_matches":entries==&desired,"import_code":null,"requires_player_apply":true,
        "addon_execution_enabled":false,"game_execution":false,"live_qualified":false,
        "preparation_status":"draft_transport_pending"});
    let bytes = serde_json::to_vec(&draft)?;
    ensure!(
        bytes.len() <= 512 * 1024,
        "Association draft exceeds size limit"
    );
    let mut fields = vec![
        "AO-SETSPEC-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        draft["kind"].as_str().unwrap().into(),
        request.set_id.to_string(),
        scalar(&draft["spec_index"]),
        scalar(&draft["spec_id"]),
        specs.len().to_string(),
    ];
    for index in 1..=specs.len() {
        fields.push(scalar(&specs[&index.to_string()]));
    }
    fields.push(entries.len().to_string());
    for (id, entry) in entries {
        fields.push(id.clone());
        fields.extend(entry_fields(entry));
    }
    fields[1] = format!("{:x}", Sha256::digest(fields.join(":").as_bytes()));
    let code = fields.join(":");
    ensure!(
        code.len() <= 512 * 1024,
        "Association import exceeds size limit"
    );
    draft["operation_id"] = json!(fields[1]);
    draft["import_code"] = json!(code);
    draft["preparation_status"] = json!("review_integration_pending");
    Ok(draft)
}

fn entry_fields(expected: &Value) -> Vec<String> {
    let mut f = Vec::new();
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
    f
}
