//! Saved inbox header amounts and wallet-only COD arithmetic. No mail actions.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, num::NonZeroU32};

pub const MAX_COPPER: u64 = 9_999_999_999_999;

fn exact_object<'a>(value: &'a Value, keys: &[&str]) -> Result<&'a serde_json::Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected mail funds object"))?;
    ensure!(
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key)),
        "Invalid mail funds fields"
    );
    Ok(object)
}

pub fn validate(value: &Value) -> Result<()> {
    exact_object(
        value,
        &["coverage", "loaded_messages", "total_messages", "messages"],
    )?;
    ensure!(
        value["coverage"] == "loaded_inbox_funds",
        "Invalid mail funds coverage"
    );
    let loaded = value["loaded_messages"]
        .as_u64()
        .filter(|n| *n <= 100)
        .ok_or_else(|| anyhow::anyhow!("Invalid loaded message count"))?;
    ensure!(
        value["total_messages"]
            .as_u64()
            .is_some_and(|n| n >= loaded && n <= u32::MAX as u64),
        "Invalid total message count"
    );
    let rows = value["messages"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid mail funds messages"))?;
    ensure!(rows.len() as u64 == loaded, "Inconsistent loaded messages");
    for slot in 1..=loaded {
        let row = rows
            .get(&slot.to_string())
            .ok_or_else(|| anyhow::anyhow!("Sparse mail funds messages"))?;
        match row["status"].as_str() {
            Some("unavailable") => {
                exact_object(row, &["status"])?;
            }
            Some("reported") => {
                exact_object(row, &["status", "money_copper", "cod_copper"])?;
                for key in ["money_copper", "cod_copper"] {
                    ensure!(
                        row[key].as_u64().is_some_and(|n| n <= MAX_COPPER),
                        "Invalid mail funds copper"
                    );
                }
            }
            _ => anyhow::bail!("Invalid mail funds status"),
        }
    }
    Ok(())
}

fn boundary() -> Value {
    json!({"mailbox_funds_spendable":false,"inventory_credit":false,
        "current_availability_verified":false,"mail_identity_continuity_verified":false,
        "game_execution":false,"live":false})
}

/// Expects a section validated during snapshot admission.
pub fn observation(section: &Value) -> Value {
    let rows = section["messages"].as_object().unwrap();
    let reported = rows
        .values()
        .filter(|row| row["status"] == "reported")
        .count();
    let money: u64 = rows
        .values()
        .filter_map(|row| row["money_copper"].as_u64())
        .sum();
    let cod: u64 = rows
        .values()
        .filter_map(|row| row["cod_copper"].as_u64())
        .sum();
    let all = reported == rows.len();
    let complete = all && section["loaded_messages"] == section["total_messages"];
    let mut result = boundary();
    for key in ["coverage", "loaded_messages", "total_messages"] {
        result[key] = section[key].clone();
    }
    result["unloaded_messages"] =
        json!(section["total_messages"].as_u64().unwrap() - rows.len() as u64);
    result["reported_message_count"] = json!(reported);
    result["unavailable_message_count"] = json!(rows.len() - reported);
    result["all_loaded_headers_reported"] = json!(all);
    result["known_money_copper"] = json!(money);
    result["known_cod_quote_copper"] = json!(cod);
    result["total_money_copper"] = json!(complete.then_some(money));
    result["total_cod_quote_copper"] = json!(complete.then_some(cod));
    result
}

fn default_limit() -> usize {
    20
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub message_slots: Option<Vec<NonZeroU32>>,
    #[serde(default)]
    pub reserve_copper: u64,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl Page {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=50).contains(&self.limit),
            "Limit must be between 1 and 50"
        );
        ensure!(self.reserve_copper <= MAX_COPPER, "Invalid reserve copper");
        if let Some(slots) = &self.message_slots {
            ensure!(
                (1..=100).contains(&slots.len())
                    && slots.iter().all(|n| n.get() <= 100)
                    && slots.iter().collect::<BTreeSet<_>>().len() == slots.len(),
                "Require 1 to 100 distinct message slots from 1 to 100"
            );
        }
        Ok(())
    }
}

pub fn review(snapshot: &Snapshot, page: &Page) -> Result<Value> {
    page.validate()?;
    ensure!(
        snapshot.client.family == Family::Forever,
        "Mail funds review requires Forever"
    );
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let spendable = wallet.map(|n| n.saturating_sub(page.reserve_copper));
    let covered = wallet.map(|n| n >= page.reserve_copper);
    let mut result = boundary();
    result["payment_eligibility_verified"] = json!(false);
    result["status"] = json!("funds_not_recorded");
    for key in [
        "rows",
        "selected_message_count",
        "reported_message_count",
        "unknown_message_count",
        "known_money_copper",
        "known_cod_quote_copper",
        "total_selected_cod_quote_copper",
        "known_cod_fits_after_reserve",
        "returned_message_count",
    ] {
        result[key] = Value::Null;
    }
    result["wallet_copper"] = json!(wallet);
    result["reserve_copper"] = json!(page.reserve_copper);
    result["reserve_covered"] = json!(covered);
    result["spendable_after_reserve_copper"] = json!(spendable);
    result["offset"] = json!(page.offset);
    result["limit"] = json!(page.limit);
    result["has_more"] = json!(false);
    let Some(section) = snapshot.sections.get("mail_funds") else {
        return Ok(result);
    };
    validate(section)?;
    let selected: BTreeSet<u32> = match &page.message_slots {
        Some(slots) => slots.iter().map(|n| n.get()).collect(),
        None => (1..=section["loaded_messages"].as_u64().unwrap() as u32).collect(),
    };
    let source = section["messages"].as_object().unwrap();
    let mut rows = Vec::new();
    let (mut money, mut cod, mut reported) = (0u64, 0u64, 0usize);
    for slot in &selected {
        let row = source.get(&slot.to_string());
        let status = row
            .and_then(|row| row["status"].as_str())
            .unwrap_or("not_captured");
        let amount = row.and_then(|row| row["money_copper"].as_u64());
        let quote = row.and_then(|row| row["cod_copper"].as_u64());
        if status == "reported" {
            reported += 1;
            money += amount.unwrap();
            cod += quote.unwrap();
        }
        rows.push(json!({"message_slot":slot,"status":status,"money_copper":amount,"cod_copper":quote,
            "cod_shortfall_after_reserve":quote.zip(spendable).map(|(quote, balance)| quote.saturating_sub(balance))}));
    }
    let unknown = selected.len() - reported;
    let fits = match (covered, spendable) {
        (Some(false), _) => Some(false),
        (Some(true), Some(balance)) if cod > balance => Some(false),
        (Some(true), Some(_)) if unknown == 0 => Some(true),
        _ => None,
    };
    let count = rows.len();
    let rows: Vec<_> = rows
        .into_iter()
        .skip(page.offset as usize)
        .take(page.limit)
        .collect();
    result["status"] = json!("reviewed");
    result["selected_message_count"] = json!(count);
    result["reported_message_count"] = json!(reported);
    result["unknown_message_count"] = json!(unknown);
    result["known_money_copper"] = json!(money);
    result["known_cod_quote_copper"] = json!(cod);
    result["total_selected_cod_quote_copper"] = json!((unknown == 0).then_some(cod));
    result["known_cod_fits_after_reserve"] = json!(fits);
    result["returned_message_count"] = json!(rows.len());
    result["has_more"] = json!((page.offset as usize).saturating_add(rows.len()) < count);
    result["rows"] = json!(rows);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(rows: Value, total: u32) -> Value {
        json!({"coverage":"loaded_inbox_funds","loaded_messages":rows.as_object().unwrap().len(),
            "total_messages":total,"messages":rows})
    }
    fn snapshot(funds: Option<Value>, wallet: Option<u64>) -> Snapshot {
        let mut snapshot: Snapshot = serde_json::from_value(json!({"schema_version":1,"source":"fixture",
            "captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"forever","build":"1","interface":1},
            "character":{"name":"Example","realm":"Test"},"sections":{}})).unwrap();
        if let Some(funds) = funds {
            snapshot.sections.insert("mail_funds".into(), funds);
        }
        if let Some(wallet) = wallet {
            snapshot
                .sections
                .insert("wallet".into(), json!({"copper":wallet}));
        }
        snapshot.validate().unwrap();
        snapshot
    }
    fn page(options: Value) -> Page {
        serde_json::from_value(options).unwrap()
    }
    fn reported(money: u64, cod: u64) -> Value {
        json!({"status":"reported","money_copper":money,"cod_copper":cod})
    }

    #[test]
    fn mail_funds_strict_shape_and_limits() {
        let valid = section(
            json!({"1":reported(MAX_COPPER,0),"2":{"status":"unavailable"}}),
            u32::MAX,
        );
        validate(&valid).unwrap();
        let mut cases = vec![
            Value::Null,
            json!({}),
            section(json!({"01":reported(0,0)}), 1),
            section(json!({"1":{"status":"unknown"}}), 1),
            section(json!({"1":{"status":"unavailable","money_copper":0}}), 1),
            section(
                json!({"1":{"status":"reported","money_copper":true,"cod_copper":0}}),
                1,
            ),
            section(
                json!({"1":{"status":"reported","money_copper":null,"cod_copper":0}}),
                1,
            ),
            section(json!({"1":reported(MAX_COPPER+1,0)}), 1),
        ];
        for (key, value) in [
            ("extra", json!(0)),
            ("loaded_messages", json!(true)),
            ("total_messages", json!(u32::MAX as u64 + 1)),
            ("coverage", json!("loaded_inbox_attachments")),
        ] {
            let mut bad = valid.clone();
            bad[key] = value;
            cases.push(bad);
        }
        let full: serde_json::Map<String, Value> = (1..=100)
            .map(|n| (n.to_string(), reported(MAX_COPPER, MAX_COPPER)))
            .collect();
        let full = section(json!(full), 100);
        validate(&full).unwrap();
        assert_eq!(observation(&full)["total_money_copper"], 100 * MAX_COPPER);
        let mut overflow = full.clone();
        overflow["loaded_messages"] = json!(101);
        cases.push(overflow);
        for invalid in cases {
            assert!(validate(&invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn mail_funds_observation_preserves_unknown_and_zero() {
        let empty = section(json!({}), 0);
        assert_eq!(observation(&empty)["total_money_copper"], 0);
        let unloaded = section(json!({}), 1);
        assert!(observation(&unloaded)["total_money_copper"].is_null());
        let mixed = section(json!({"1":reported(20,30),"2":{"status":"unavailable"}}), 2);
        let observed = observation(&mixed);
        assert_eq!(observed["known_money_copper"], 20);
        assert_eq!(observed["reported_message_count"], 1);
        assert_eq!(observed["unavailable_message_count"], 1);
        assert!(observed["total_cod_quote_copper"].is_null());
        assert_eq!(observed["mailbox_funds_spendable"], false);
    }

    #[test]
    fn mail_funds_wallet_budget_excludes_pending_money() {
        let saved = snapshot(
            Some(section(
                json!({"1":reported(MAX_COPPER,60),"2":reported(0,20)}),
                2,
            )),
            Some(100),
        );
        let result = review(&saved, &page(json!({"reserve_copper":30}))).unwrap();
        assert_eq!(result["spendable_after_reserve_copper"], 70);
        assert_eq!(result["known_cod_fits_after_reserve"], false);
        assert_eq!(result["rows"][0]["cod_shortfall_after_reserve"], 0);
        assert_eq!(result["known_money_copper"], MAX_COPPER);
        let result = review(&saved, &page(json!({"reserve_copper":20}))).unwrap();
        assert_eq!(result["known_cod_fits_after_reserve"], true);
        assert_eq!(result["payment_eligibility_verified"], false);
        let result = review(&saved, &page(json!({"reserve_copper":101}))).unwrap();
        assert_eq!(result["reserve_covered"], false);
        assert_eq!(result["rows"][0]["cod_shortfall_after_reserve"], 60);
    }

    #[test]
    fn mail_funds_selection_unknown_reserve_and_pagination() {
        let funds = section(
            json!({"1":reported(1,5),"2":{"status":"unavailable"},"3":reported(2,10)}),
            5,
        );
        let saved = snapshot(Some(funds.clone()), Some(100));
        let result = review(
            &saved,
            &page(json!({"message_slots":[100,3,1,2],"offset":1,"limit":1})),
        )
        .unwrap();
        assert_eq!(result["selected_message_count"], 4);
        assert_eq!(result["known_cod_quote_copper"], 15);
        assert_eq!(result["unknown_message_count"], 2);
        assert!(result["total_selected_cod_quote_copper"].is_null());
        assert!(result["known_cod_fits_after_reserve"].is_null());
        assert_eq!(result["rows"][0]["message_slot"], 2);
        assert!(result["rows"][0]["money_copper"].is_null());
        assert_eq!(result["has_more"], true);
        let outside = review(&saved, &page(json!({"message_slots":[100]}))).unwrap();
        assert_eq!(outside["rows"][0]["status"], "not_captured");
        let underfunded =
            review(&snapshot(Some(funds.clone()), Some(0)), &page(json!({}))).unwrap();
        assert_eq!(underfunded["known_cod_fits_after_reserve"], false);
        let unknown_wallet = review(&snapshot(Some(funds), None), &page(json!({}))).unwrap();
        assert!(unknown_wallet["known_cod_fits_after_reserve"].is_null());
        assert!(unknown_wallet["rows"][0]["cod_shortfall_after_reserve"].is_null());
    }

    #[test]
    fn mail_funds_missing_empty_family_and_options() {
        let missing = review(
            &snapshot(None, Some(99)),
            &page(json!({"message_slots":[1]})),
        )
        .unwrap();
        assert_eq!(missing["status"], "funds_not_recorded");
        for key in [
            "rows",
            "selected_message_count",
            "known_money_copper",
            "unknown_message_count",
            "returned_message_count",
        ] {
            assert!(missing[key].is_null());
        }
        assert_eq!(missing["wallet_copper"], 99);
        let saved = snapshot(Some(section(json!({}), 0)), Some(0));
        let empty = review(&saved, &page(json!({}))).unwrap();
        assert_eq!(empty["rows"], json!([]));
        assert_eq!(empty["selected_message_count"], 0);
        assert_eq!(empty["known_cod_fits_after_reserve"], true);
        let past_end = review(&saved, &page(json!({"offset":u32::MAX}))).unwrap();
        assert_eq!(past_end["has_more"], false);
        let mut retail = saved.clone();
        retail.client.family = Family::Retail;
        assert!(retail.validate().is_err());
        assert!(review(&retail, &page(json!({}))).is_err());
        for options in [
            json!({"limit":0}),
            json!({"limit":51}),
            json!({"message_slots":[]}),
            json!({"message_slots":[1,1]}),
            json!({"message_slots":[101]}),
            json!({"reserve_copper":MAX_COPPER+1}),
        ] {
            assert!(page(options).validate().is_err());
        }
        for options in [
            json!({"other":0}),
            json!({"limit":null}),
            json!({"reserve_copper":true}),
            json!({"message_slots":[0]}),
            json!({"message_slots":[false]}),
        ] {
            assert!(serde_json::from_value::<Page>(options).is_err());
        }
        let mut invalid = saved;
        invalid.sections.get_mut("mail_funds").unwrap()["messages"] = Value::Null;
        assert!(review(&invalid, &page(json!({}))).is_err());
    }
}
