//! Historical item index for explicit owner-local captures. Never pooled inventory.
use crate::{Character, Snapshot};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Observation {
    id: String,
    character: Character,
    captured_at: DateTime<Utc>,
    quantities: Value,
}

impl Observation {
    /// Only called after the owner-store read and complete item search succeed.
    pub fn new(id: &str, snapshot: &Snapshot, result: &Value) -> Self {
        Self {id:id.into(),character:snapshot.character.clone(),captured_at:snapshot.captured_at,
            quantities:result["quantity_by_storage"].clone()}
    }
}

pub fn review(observations: Vec<Observation>, item_id: u32) -> Value {
    let selected = observations.len();
    let primary = observations.first().map(|row|(row.character.name.clone(),row.character.realm.clone()));
    let mut grouped = BTreeMap::<(String,String),Vec<Observation>>::new();
    for row in observations {
        grouped.entry((row.character.name.clone(),row.character.realm.clone())).or_default().push(row);
    }
    let mut rows = Vec::new();
    let mut ambiguous = 0;
    for (identity,mut captures) in grouped {
        captures.sort_by(|a,b|b.captured_at.cmp(&a.captured_at).then(a.id.cmp(&b.id)));
        let latest = &captures[0];
        let candidates: Vec<_> = captures.iter().take_while(|row|row.captured_at==latest.captured_at).collect();
        let unique = candidates.len()==1;
        if !unique {ambiguous+=1;}
        let quantities = unique.then(||json!({"equipment":latest.quantities["equipment"],
            "bags":latest.quantities["bags"],"character_bank":latest.quantities["character_bank"]}));
        let missing = unique.then(||["equipment","bags","character_bank"].into_iter()
            .filter(|storage|latest.quantities[*storage].is_null()).collect::<Vec<_>>());
        rows.push(json!({"character":latest.character,"primary_character":primary.as_ref()==Some(&identity),
            "selected_capture_count":captures.len(),"latest_captured_at":latest.captured_at,
            "latest_capture_status":if unique {"unique_latest"} else {"ambiguous_latest"},
            "latest_snapshot_id":unique.then_some(&latest.id),
            "latest_candidate_snapshot_ids":candidates.iter().map(|row|&row.id).collect::<Vec<_>>(),
            "older_selected_capture_count":captures.len()-candidates.len(),
            "quantity_by_character_storage":quantities,"unobserved_character_storage":missing,
            "account_bank_quantity":if unique {latest.quantities["account_bank"].clone()} else {Value::Null}}));
    }
    rows.sort_by(|a,b|b["primary_character"].as_bool().cmp(&a["primary_character"].as_bool())
        .then(a["character"]["name"].as_str().cmp(&b["character"]["name"].as_str()))
        .then(a["character"]["realm"].as_str().cmp(&b["character"]["realm"].as_str())));
    json!({"basis":"explicit_selected_captures_unique_latest_per_exact_character",
        "identity_basis":"exact_saved_name_and_realm","item_id":item_id,
        "selected_capture_count":selected,"character_count":rows.len(),"ambiguous_character_count":ambiguous,
        "characters":rows,"missing_storage_falls_back_to_older_capture":false,
        "account_bank_quantities_are_additive":false,"account_identity_verified":false,
        "cross_character_total":null,"simultaneous_ownership_verified":false,"transfers_reconciled":false,
        "all_characters_covered":false,"current_inventory_verified":false,"active_character_verified":false,
        "item_variant_equivalence_verified":false,"game_execution":false,"live":false})
}
