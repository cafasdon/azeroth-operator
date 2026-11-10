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
