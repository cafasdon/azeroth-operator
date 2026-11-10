//! Saved quest-giver provider references. No native identity or availability claims.
use anyhow::{Result, ensure};
use serde_json::{Map, Value, json};

fn object<'a>(
    value: &'a Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>> {
    let map = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected quest reference object"))?;
    ensure!(
        required.iter().all(|field| map.contains_key(*field)),
        "Missing quest reference field"
    );
    ensure!(
        map.keys()
            .all(|field| required.contains(&field.as_str()) || optional.contains(&field.as_str())),
        "Unknown quest reference field"
    );
    Ok(map)
}

fn id(key: &str) -> Result<()> {
    let value: u32 = key.parse()?;
    ensure!(
        value > 0 && value.to_string() == key,
        "Invalid quest reference ID"
    );
    Ok(())
}

fn count(value: &Value) -> Result<u64> {
    let count = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid quest reference count"))?;
    ensure!(count <= 4096, "Quest reference count out of bounds");
    Ok(count)
}

fn group(value: &Value) -> Result<()> {
    object(value, &["entries", "distinct_id_count", "truncated"], &[])?;
    let total = count(&value["distinct_id_count"])?;
    let entries = value["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected quest reference entries"))?;
    ensure!(
        entries.len() == total.min(16) as usize && value["truncated"].as_bool() == Some(total > 16),
        "Invalid quest reference truncation"
    );
    for (key, flag) in entries {
        id(key)?;
        ensure!(
            flag.as_bool() == Some(true),
            "Invalid quest reference presence flag"
        );
    }
    Ok(())
}

pub fn validate(value: &Value) -> Result<()> {
    object(
        value,
        &[
            "coverage",
            "provider",
            "provider_mode",
            "provider_contract_version",
            "reported_quest_count",
            "entries",
        ],
        &[],
    )?;
    ensure!(
        value["coverage"] == "first_saved_quest_ids"
            && value["provider"] == "questiedb"
            && matches!(value["provider_mode"].as_str(), Some("source" | "baked"))
            && value["provider_contract_version"]
                .as_u64()
                .is_some_and(|version| version > 0 && version <= u32::MAX as u64),
        "Invalid quest reference provider contract"
    );
    let total = count(&value["reported_quest_count"])?;
    let entries = value["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected quest reference entries"))?;
    ensure!(
        entries.len() == total.min(64) as usize,
        "Invalid quest reference row count"
    );
    for (key, row) in entries {
        id(key)?;
        match row["status"].as_str() {
            Some("unavailable" | "not_in_provider") => {
                object(row, &["status"], &[])?;
            }
            Some("reported") => {
                object(row, &["status"], &["started_by", "finished_by"])?;
                for (field, groups) in [
                    ("started_by", &["npcs", "objects", "items"][..]),
                    ("finished_by", &["npcs", "objects"][..]),
                ] {
                    if let Some(value) = row.get(field) {
                        let sources = object(value, &[], groups)?;
                        for source in sources.values() {
                            group(source)?;
                        }
                    }
                }
            }
            _ => anyhow::bail!("Invalid quest reference status"),
        }
    }
    Ok(())
}

/// Match the bounded reference capture to the same export's saved quest IDs.
pub fn validate_source(reference: &Value, quests: Option<&Value>) -> Result<()> {
    let quests = quests
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Quest references require saved quests"))?;
    ensure!(
        quests.len() <= 4096,
        "Too many saved quests for quest references"
    );
    ensure!(
        reference["reported_quest_count"].as_u64() == Some(quests.len() as u64),
        "Quest reference source count mismatch"
    );
    let mut saved_ids = std::collections::BTreeSet::new();
    for key in quests.keys() {
        id(key)?;
        saved_ids.insert(key.parse::<u32>()?);
    }
    let entries = reference["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected quest reference entries"))?;
    let mut reference_ids = std::collections::BTreeSet::new();
    for key in entries.keys() {
        id(key)?;
        reference_ids.insert(key.parse::<u32>()?);
    }
    ensure!(
        reference_ids.into_iter().eq(saved_ids.into_iter().take(64)),
        "Quest references must match first saved quest IDs"
    );
    Ok(())
}

/// Input is a validated section. Counts describe only saved provider observations.
pub fn observation(value: &Value) -> Value {
    let entries = value["entries"].as_object().unwrap();
    let statuses: Map<String, Value> = ["reported", "unavailable", "not_in_provider"]
        .into_iter()
        .map(|status| {
            (
                status.into(),
                json!(
                    entries
                        .values()
                        .filter(|row| row["status"] == status)
                        .count()
                ),
            )
        })
        .collect();
    json!({"reported_quest_count":value["reported_quest_count"], "captured_quest_count":entries.len(),
        "status_counts":statuses, "current_availability_verified":false, "native_identity_verified":false,
        "complete_world_data_verified":false, "game_execution":false})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section() -> Value {
        json!({"coverage":"first_saved_quest_ids","provider":"questiedb","provider_mode":"source",
            "provider_contract_version":1,"reported_quest_count":0,"entries":{}})
    }

    fn reported() -> Value {
        let mut value = section();
        value["reported_quest_count"] = json!(1);
        value["entries"] = json!({"42":{"status":"reported","started_by":{},"finished_by":{}}});
        value
    }

    #[test]
    fn empty_and_optional_groups() {
        validate(&section()).unwrap();
        let mut value = reported();
        validate(&value).unwrap();
        value["entries"]["42"]["started_by"] = json!({"items":{"entries":{"4294967295":true},"distinct_id_count":1,"truncated":false},"npcs":{"entries":{},"distinct_id_count":0,"truncated":false}});
        value["provider_mode"] = json!("baked");
        value["provider_contract_version"] = json!(u32::MAX);
        validate(&value).unwrap();
        let summary = observation(&value);
        assert_eq!(summary["status_counts"]["reported"], 1);
        for field in [
            "current_availability_verified",
            "native_identity_verified",
            "complete_world_data_verified",
            "game_execution",
        ] {
            assert_eq!(summary[field], false);
        }
        assert!(summary.get("entries").is_none());
    }

    #[test]
    fn truncation_bounds() {
        let mut value = section();
        value["reported_quest_count"] = json!(4096);
        value["entries"] = Value::Object(
            (1..=64)
                .map(|id| (id.to_string(), json!({"status":"unavailable"})))
                .collect(),
        );
        validate(&value).unwrap();
        value["entries"]["1"] = json!({"status":"reported","started_by":{"npcs":{"entries":Value::Object((1..=16).map(|id| (id.to_string(), json!(true))).collect()),"distinct_id_count":4096,"truncated":true}}});
        validate(&value).unwrap();
        value["entries"]["1"]["started_by"]["npcs"]["truncated"] = json!(false);
        assert!(validate(&value).is_err());
        value["reported_quest_count"] = json!(4097);
        assert!(validate(&value).is_err());
    }

    #[test]
    fn rejects_contract_fields_types_and_counts() {
        for (field, bad) in [
            ("coverage", json!("all")),
            ("provider", json!("other")),
            ("provider_mode", json!("live")),
            ("provider_contract_version", json!(0)),
            ("provider_contract_version", json!(4294967296u64)),
            ("provider_contract_version", json!(1.0)),
            ("reported_quest_count", json!(-1)),
            ("reported_quest_count", json!(1)),
            ("reported_quest_count", json!(0.0)),
            ("entries", json!([])),
            ("extra", json!(true)),
        ] {
            let mut value = section();
            value[field] = bad;
            assert!(validate(&value).is_err(), "accepted {field}: {value}");
        }
        let mut value = section();
        value.as_object_mut().unwrap().remove("provider");
        assert!(validate(&value).is_err());
    }

    #[test]
    fn rejects_rows_groups_and_ids() {
        for row in [
            json!({"status":"unknown"}),
            json!({"status":"unavailable","started_by":{}}),
            json!({"status":"not_in_provider","extra":true}),
            json!({"status":"reported","started_by":null}),
            json!({"status":"reported","finished_by":{"items":{}}}),
            json!({"status":"reported","started_by":{"extra":{}}}),
            json!(null),
        ] {
            let mut value = reported();
            value["entries"]["42"] = row;
            assert!(validate(&value).is_err(), "accepted {value}");
        }
        for key in ["0", "01", "+1", "4294967296", "x"] {
            let mut value = reported();
            value["entries"] = json!({key:{"status":"reported"}});
            assert!(validate(&value).is_err(), "accepted {key}");
        }
        for bad in [
            json!({"entries":{"1":false},"distinct_id_count":1,"truncated":false}),
            json!({"entries":{"01":true},"distinct_id_count":1,"truncated":false}),
            json!({"entries":{},"distinct_id_count":1,"truncated":false}),
            json!({"entries":{},"distinct_id_count":4097,"truncated":true}),
            json!({"entries":{},"distinct_id_count":0,"truncated":false,"extra":0}),
            json!({"entries":{},"distinct_id_count":0,"truncated":0}),
            json!(null),
        ] {
            let mut value = reported();
            value["entries"]["42"]["started_by"] = json!({"npcs":bad});
            assert!(validate(&value).is_err(), "accepted {value}");
        }
        for status in ["unavailable", "not_in_provider"] {
            let mut value = reported();
            value["entries"]["42"] = json!({"status":status});
            validate(&value).unwrap();
            assert_eq!(observation(&value)["status_counts"][status], 1);
        }
    }

    #[test]
    fn source_requires_known_matching_saved_quests() {
        let empty = section();
        validate_source(&empty, Some(&json!({}))).unwrap();
        assert!(validate_source(&empty, None).is_err());
        assert!(validate_source(&empty, Some(&json!([]))).is_err());
        assert!(validate_source(&empty, Some(&json!({"42":{}}))).is_err());
        let value = reported();
        validate_source(&value, Some(&json!({"42":{}}))).unwrap();
        assert!(validate_source(&value, Some(&json!({"43":{}}))).is_err());
        for key in ["0", "01", "+1", "4294967296", "x"] {
            assert!(validate_source(&value, Some(&json!({key:{}}))).is_err());
        }
        let oversized = Value::Object((1..=4097).map(|id| (id.to_string(), json!({}))).collect());
        assert!(validate_source(&value, Some(&oversized)).is_err());
    }

    #[test]
    fn source_requires_numeric_first_64() {
        let quests = Value::Object((1..=66).map(|id| (id.to_string(), json!({}))).collect());
        let mut value = section();
        value["reported_quest_count"] = json!(66);
        value["entries"] = Value::Object(
            (1..=64)
                .map(|id| (id.to_string(), json!({"status":"unavailable"})))
                .collect(),
        );
        validate(&value).unwrap();
        validate_source(&value, Some(&quests)).unwrap();
        let row = value["entries"]
            .as_object_mut()
            .unwrap()
            .remove("64")
            .unwrap();
        value["entries"]["65"] = row;
        validate(&value).unwrap();
        assert!(validate_source(&value, Some(&quests)).is_err());
    }
}
