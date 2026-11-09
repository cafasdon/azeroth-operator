//! Same-row filtering of historical base skill ranks; no training or simulation.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::num::NonZeroU32;

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillLineFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_line_id: Option<NonZeroU32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<NonZeroU32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_rank: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_rank: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_cap: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_cap: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_points_to_cap: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_points_to_cap: Option<u32>,
    /// Exact reported modifier, separate from base rank.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<i32>,
    /// Exact reported temporary points, separate from base rank and modifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_points: Option<i32>,
}

fn twenty() -> usize {
    20
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillLinePage {
    pub filter: SkillLineFilter,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "twenty")]
    pub limit: usize,
}

impl SkillLineFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.skill_line_id.is_some()
                || self.category_id.is_some()
                || self.minimum_rank.is_some()
                || self.maximum_rank.is_some()
                || self.minimum_cap.is_some()
                || self.maximum_cap.is_some()
                || self.minimum_points_to_cap.is_some()
                || self.maximum_points_to_cap.is_some()
                || self.modifier.is_some()
                || self.temporary_points.is_some(),
            "Nonempty skill-line filter required"
        );
        for (minimum, maximum) in [
            (self.minimum_rank, self.maximum_rank),
            (self.minimum_cap, self.maximum_cap),
            (self.minimum_points_to_cap, self.maximum_points_to_cap),
        ] {
            ensure!(
                minimum
                    .zip(maximum)
                    .is_none_or(|(minimum, maximum)| minimum <= maximum),
                "Skill-line bounds reversed"
            );
        }
        Ok(())
    }
    fn matches(&self, id: u32, row: &Value) -> Option<bool> {
        let rank = row["rank"].as_u64().unwrap();
        let cap = row["max_rank"].as_u64().unwrap();
        let gap = cap - rank;
        if self.skill_line_id.is_some_and(|wanted| wanted.get() != id)
            || self
                .minimum_rank
                .is_some_and(|minimum| rank < u64::from(minimum))
            || self
                .maximum_rank
                .is_some_and(|maximum| rank > u64::from(maximum))
            || self
                .minimum_cap
                .is_some_and(|minimum| cap < u64::from(minimum))
            || self
                .maximum_cap
                .is_some_and(|maximum| cap > u64::from(maximum))
            || self
                .minimum_points_to_cap
                .is_some_and(|minimum| gap < u64::from(minimum))
            || self
                .maximum_points_to_cap
                .is_some_and(|maximum| gap > u64::from(maximum))
        {
            return Some(false);
        }
        let mut unknown = false;
        for (field, expected) in [
            (
                "category_id",
                self.category_id.map(|id| i64::from(id.get())),
            ),
            ("modifier", self.modifier.map(i64::from)),
            ("temporary_points", self.temporary_points.map(i64::from)),
        ] {
            if let Some(expected) = expected {
                match row[field].as_i64() {
                    Some(actual) if actual != expected => return Some(false),
                    Some(_) => {}
                    None => unknown = true,
                }
            }
        }
        (!unknown).then_some(true)
    }
    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        let found = scan(snapshot.sections.get("skill_lines")?, self).ok()?;
        if found.rows.is_empty() {
            return None;
        }
        let mut result = metadata();
        result["filter"] = json!(self);
        result["captured_skill_line_count"] = json!(found.captured);
        result["matching_skill_line_count"] = json!(found.rows.len());
        result["unknown_skill_line_count"] = json!(found.unknown);
        result["nonmatching_skill_line_count"] =
            json!(found.captured - found.unknown - found.rows.len());
        result["matches_truncated"] = json!(found.rows.len() > 20);
        result["matching_skill_lines"] = json!(found.rows.into_iter().take(20).collect::<Vec<_>>());
        Some(result)
    }
}

impl SkillLinePage {
    pub fn validate(&self) -> Result<()> {
        self.filter.validate()?;
        ensure!(
            (1..=50).contains(&self.limit),
            "Skill-line page limit out of range"
        );
        Ok(())
    }
}

struct Matches {
    captured: usize,
    unknown: usize,
    rows: Vec<Value>,
}
fn scan(section: &Value, filter: &SkillLineFilter) -> Result<Matches> {
    crate::schema::validate("skill_lines", section)?;
    let entries = section.as_object().unwrap();
    let mut unknown = 0;
    let mut rows = Vec::new();
    for (id, row) in entries {
        let id = id.parse::<u32>()?;
        match filter.matches(id,row) {
            Some(true)=>rows.push(json!({"skill_line_id":id,"observation":row,
                "points_to_observed_cap":row["max_rank"].as_u64().unwrap()-row["rank"].as_u64().unwrap()})),
            Some(false)=>{},None=>unknown+=1,
        }
    }
    rows.sort_unstable_by_key(|row| row["skill_line_id"].as_u64().unwrap());
    Ok(Matches {
        captured: entries.len(),
        unknown,
        rows,
    })
}
fn metadata() -> Value {
    json!({"basis":"captured_skill_lines_only","rank_basis":"base_rank_only",
    "complete_skill_coverage_verified":false,"current_training_eligibility_verified":false,
    "effective_rank_inferred":false,"simulator_input_modified":false,"learning_state_inferred":false,
    "game_execution":false,"live":false})
}

pub fn inspect(section: Option<&Value>, page: &SkillLinePage) -> Result<Value> {
    page.validate()?;
    let mut result = metadata();
    result["filter"] = json!(page.filter);
    result["offset"] = json!(page.offset);
    result["limit"] = json!(page.limit);
    let Some(section) = section else {
        for field in [
            "rows",
            "captured_count",
            "matching_count",
            "unknown_count",
            "nonmatching_count",
            "has_more",
        ] {
            result[field] = Value::Null;
        }
        return Ok(result);
    };
    let found = scan(section, &page.filter)?;
    let matched = found.rows.len();
    result["captured_count"] = json!(found.captured);
    result["matching_count"] = json!(matched);
    result["unknown_count"] = json!(found.unknown);
    result["nonmatching_count"] = json!(found.captured - matched - found.unknown);
    result["has_more"] = json!(page.offset.saturating_add(page.limit) < matched);
    result["rows"] = json!(
        found
            .rows
            .into_iter()
            .skip(page.offset)
            .take(page.limit)
            .collect::<Vec<_>>()
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(filter: Value) -> SkillLinePage {
        serde_json::from_value(json!({"filter":filter})).unwrap()
    }
    #[test]
    fn same_row_unknowns_and_base_rank_only() {
        let rows = json!({"10":{"name":"A","rank":200,"max_rank":300,"modifier":100,"temporary_points":50},
            "2":{"name":"B","rank":20,"max_rank":100,"category_id":9},
            "3":{"name":"C","rank":300,"max_rank":300,"category_id":9}});
        let filtered = inspect(
            Some(&rows),
            &page(json!({"category_id":9,"minimum_rank":100})),
        )
        .unwrap();
        assert_eq!(filtered["matching_count"], 1);
        assert_eq!(filtered["unknown_count"], 1);
        assert_eq!(filtered["nonmatching_count"], 1);
        assert_eq!(filtered["rows"][0]["skill_line_id"], 3);
        let filtered = inspect(Some(&rows), &page(json!({"minimum_points_to_cap":1}))).unwrap();
        assert_eq!(filtered["rows"][0]["skill_line_id"], 2);
        assert_eq!(filtered["rows"][1]["skill_line_id"], 10);
        assert_eq!(filtered["rows"][1]["points_to_observed_cap"], 100);
    }
    #[test]
    fn missing_empty_and_huge_offset() {
        let mut selected = page(json!({"minimum_rank":0}));
        assert!(inspect(None, &selected).unwrap()["rows"].is_null());
        assert_eq!(
            inspect(Some(&json!({})), &selected).unwrap()["rows"],
            json!([])
        );
        selected.offset = usize::MAX;
        let found = inspect(
            Some(&json!({"1":{"name":"A","rank":0,"max_rank":u32::MAX}})),
            &selected,
        )
        .unwrap();
        assert_eq!(found["has_more"], false);
        assert_eq!(found["matching_count"], 1);
        assert_eq!(found["rows"], json!([]));
    }
    #[test]
    fn empty_reversed_and_invalid_paging_reject() {
        for filter in [
            json!({}),
            json!({"minimum_rank":2,"maximum_rank":1}),
            json!({"minimum_cap":2,"maximum_cap":1}),
            json!({"minimum_points_to_cap":2,"maximum_points_to_cap":1}),
        ] {
            assert!(page(filter).validate().is_err());
        }
        for limit in [0, 51] {
            let mut selected = page(json!({"minimum_rank":0}));
            selected.limit = limit;
            assert!(selected.validate().is_err());
        }
    }
}
