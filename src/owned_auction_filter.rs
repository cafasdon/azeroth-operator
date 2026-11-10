//! Search saved loaded owned-auction rows; no auction actions or stock credit.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::num::{NonZeroU32, NonZeroU64};

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnedAuctionFilter {
    pub item_id: Option<NonZeroU32>,
    pub auction_id: Option<NonZeroU64>,
    /// Exact uninterpreted status from the saved cache.
    pub reported_status: Option<u32>,
}

fn twenty() -> usize {
    20
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnedAuctionPage {
    pub filter: OwnedAuctionFilter,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "twenty")]
    pub limit: usize,
}

impl OwnedAuctionFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.item_id.is_some() || self.auction_id.is_some() || self.reported_status.is_some(),
            "Nonempty owned auction filter required"
        );
        ensure!(
            self.auction_id
                .is_none_or(|id| id.get() <= 9_007_199_254_740_991),
            "Auction ID exceeds safe integer range"
        );
        Ok(())
    }

    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        let rows = scan(snapshot.sections.get("owned_auctions")?, self).ok()?;
        if rows.is_empty() {
            return None;
        }
        let mut result = metadata();
        result["filter"] = json!(self);
        result["matching_auction_count"] = json!(rows.len());
        result["matching_reported_item_units"] = json!(units(&rows));
        result["matches_truncated"] = json!(rows.len() > 20);
        result["matching_auctions"] = json!(rows.into_iter().take(20).collect::<Vec<_>>());
        Some(result)
    }
}

impl OwnedAuctionPage {
    pub fn validate(&self) -> Result<()> {
        self.filter.validate()?;
        ensure!(
            (1..=50).contains(&self.limit),
            "Owned auction page limit out of range"
        );
        Ok(())
    }
}

fn metadata() -> Value {
    json!({"basis":"loaded_player_owned_auction_cache", "status_interpretation":"none",
        "complete_auction_coverage_verified":false,"current_ownership_verified":false,
        "sale_or_delivery_inferred":false,"counts_toward_inventory":false,
        "counts_toward_wallet":false,"game_execution":false,"live":false})
}

fn units(rows: &[Value]) -> u64 {
    rows.iter()
        .map(|row| row["observation"]["quantity"].as_u64().unwrap())
        .sum()
}

fn scan(section: &Value, filter: &OwnedAuctionFilter) -> Result<Vec<Value>> {
    filter.validate()?;
    crate::schema::validate("owned_auctions", section)?;
    let mut rows = Vec::new();
    for (slot, row) in section["entries"].as_object().unwrap() {
        if filter
            .item_id
            .is_none_or(|id| row["item_id"].as_u64() == Some(u64::from(id.get())))
            && filter
                .auction_id
                .is_none_or(|id| row["auction_id"].as_u64() == Some(id.get()))
            && filter
                .reported_status
                .is_none_or(|status| row["reported_status"].as_u64() == Some(u64::from(status)))
        {
            rows.push(json!({"slot":slot.parse::<u32>()?,"observation":row}));
        }
    }
    rows.sort_unstable_by_key(|row| row["slot"].as_u64().unwrap());
    Ok(rows)
}

pub fn inspect(section: Option<&Value>, page: &OwnedAuctionPage) -> Result<Value> {
    page.validate()?;
    let mut result = metadata();
    result["filter"] = json!(page.filter);
    result["offset"] = json!(page.offset);
    result["limit"] = json!(page.limit);
    let Some(section) = section else {
        for field in [
            "captured_auction_count",
            "matching_auction_count",
            "matching_reported_item_units",
            "has_more",
            "rows",
        ] {
            result[field] = Value::Null;
        }
        return Ok(result);
    };
    let rows = scan(section, &page.filter)?;
    result["captured_auction_count"] = json!(section["entries"].as_object().unwrap().len());
    result["matching_auction_count"] = json!(rows.len());
    result["matching_reported_item_units"] = json!(units(&rows));
    result["has_more"] = json!(page.offset.saturating_add(page.limit) < rows.len());
    result["rows"] = json!(
        rows.into_iter()
            .skip(page.offset)
            .take(page.limit)
            .collect::<Vec<_>>()
    );
    Ok(result)
}

/// Compare numeric cache identities only; disappearance never proves a sale.
pub fn compare(before: Option<&Value>, after: Option<&Value>) -> Result<Value> {
    use std::collections::BTreeMap;
    let index = |section: Option<&Value>| -> Result<Option<BTreeMap<u64, Value>>> {
        section
            .map(|section| {
                crate::schema::validate("owned_auctions", section)?;
                Ok(section["entries"]
                    .as_object()
                    .unwrap()
                    .values()
                    .map(|row| (row["auction_id"].as_u64().unwrap(), row.clone()))
                    .collect())
            })
            .transpose()
    };
    let before = index(before)?;
    let after = index(after)?;
    let summary = |rows: &Option<BTreeMap<u64, Value>>| {
        rows.as_ref().map(|rows| json!({
        "captured_auction_count":rows.len(),
        "reported_item_units":rows.values().map(|row|row["quantity"].as_u64().unwrap()).sum::<u64>()
    }))
    };
    let mut result = metadata();
    result["numeric_auction_identity_verified"] = json!(false);
    result["slot_reordering_ignored"] = json!(true);
    result["before"] = json!(summary(&before));
    result["after"] = json!(summary(&after));
    let (Some(before), Some(after)) = (before, after) else {
        for field in [
            "only_before",
            "only_after",
            "changed",
            "unchanged_auction_id_count",
            "reported_item_unit_delta",
        ] {
            result[field] = Value::Null;
        }
        return Ok(result);
    };
    let bounded = |rows: Vec<Value>| {
        json!({"count":rows.len(),"truncated":rows.len()>50,
        "rows":rows.into_iter().take(50).collect::<Vec<_>>()})
    };
    result["only_before"] = bounded(
        before
            .iter()
            .filter(|(id, _)| !after.contains_key(id))
            .map(|(_, row)| row.clone())
            .collect(),
    );
    result["only_after"] = bounded(
        after
            .iter()
            .filter(|(id, _)| !before.contains_key(id))
            .map(|(_, row)| row.clone())
            .collect(),
    );
    let mut changed = Vec::new();
    let mut unchanged = 0;
    for (id, old) in &before {
        let Some(new) = after.get(id) else {
            continue;
        };
        let mut fields = serde_json::Map::new();
        for field in ["item_id", "quantity", "reported_status"] {
            if old[field] != new[field] {
                let mut change = json!({"before":old[field],"after":new[field]});
                if field == "quantity" {
                    change["delta"] = json!(
                        new[field].as_u64().unwrap() as i64 - old[field].as_u64().unwrap() as i64
                    );
                }
                fields.insert(field.into(), change);
            }
        }
        if fields.is_empty() {
            unchanged += 1;
        } else {
            changed.push(json!({"auction_id":id,"fields":fields}));
        }
    }
    result["changed"] = bounded(changed);
    result["unchanged_auction_id_count"] = json!(unchanged);
    let total = |rows: &BTreeMap<u64, Value>| {
        rows.values()
            .map(|row| row["quantity"].as_u64().unwrap())
            .sum::<u64>()
    };
    result["reported_item_unit_delta"] = json!(total(&after) as i64 - total(&before) as i64);
    Ok(result)
}
