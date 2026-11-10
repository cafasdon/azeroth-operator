//! Owner-local raw PvP flag analysis, without safety classification or actions.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const FLAGS: [&str; 3] = ["is_pvp", "is_free_for_all", "is_sanctuary"];

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerPvpFlagFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pvp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_free_for_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_sanctuary: Option<bool>,
}

impl PlayerPvpFlagFilter {
    fn predicates(&self) -> [(&'static str, Option<bool>); 3] {
        [
            ("is_pvp", self.is_pvp),
            ("is_free_for_all", self.is_free_for_all),
            ("is_sanctuary", self.is_sanctuary),
        ]
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.predicates().iter().any(|(_, value)| value.is_some()),
            "Nonempty player PvP flags filter required"
        );
        Ok(())
    }
    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        self.validate().ok()?;
        let section = snapshot.sections.get("player_pvp_flags")?;
        crate::schema::validate("player_pvp_flags", section).ok()?;
        if !self.predicates().into_iter().all(|(field, expected)| {
            expected.is_none_or(|expected| section[field].as_bool() == Some(expected))
        }) {
            return None;
        }
        let mut result = observation(section);
        result["filter"] = json!(self);
        Some(result)
    }
}

fn boundary() -> Value {
    json!({"basis":"captured_player_pvp_flags_only","current_pvp_state_verified":false,
        "character_continuity_verified":false,"combat_eligibility_verified":false,
        "zone_safety_verified":false,"pvp_event_inferred":false,"game_execution":false,"live":false})
}

/// Input is a validated snapshot section; no relationships between flags are inferred.
pub fn observation(section: &Value) -> Value {
    let mut result = boundary();
    result["coverage"] = section["coverage"].clone();
    result["reported_flags"] = Value::Object(
        FLAGS
            .into_iter()
            .map(|field| (field.into(), section[field].clone()))
            .collect(),
    );
    result
}

pub fn compare(before: Option<&Value>, after: Option<&Value>) -> Result<Value> {
    for section in [before, after].into_iter().flatten() {
        crate::schema::validate("player_pvp_flags", section)?;
    }
    let mut result = boundary();
    result["before"] = json!(before.map(observation));
    result["after"] = json!(after.map(observation));
    let (Some(before), Some(after)) = (before, after) else {
        result["flag_changes"] = Value::Null;
        result["changed_flag_count"] = Value::Null;
        return Ok(result);
    };
    let changes: serde_json::Map<String, Value> = FLAGS
        .into_iter()
        .filter(|field| before[*field] != after[*field])
        .map(|field| {
            (
                field.into(),
                json!({"before":before[field],"after":after[field]}),
            )
        })
        .collect();
    result["changed_flag_count"] = json!(changes.len());
    result["flag_changes"] = Value::Object(changes);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(mask: u8) -> Value {
        json!({"coverage":"reported_player_pvp_flags", "is_pvp":mask & 1 != 0,
            "is_free_for_all":mask & 2 != 0,"is_sanctuary":mask & 4 != 0})
    }

    fn snapshot(section: Option<Value>) -> Snapshot {
        let sections = section
            .map(|value| {
                [("player_pvp_flags".to_string(), value)]
                    .into_iter()
                    .collect::<serde_json::Map<_, _>>()
            })
            .unwrap_or_default();
        serde_json::from_value(json!({"schema_version":1,"source":"fixture",
            "captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Fixture","realm":"Test"},"sections":sections}))
        .unwrap()
    }

    fn assert_boundary(value: &Value) {
        assert_eq!(value["basis"], "captured_player_pvp_flags_only");
        for field in [
            "current_pvp_state_verified",
            "character_continuity_verified",
            "combat_eligibility_verified",
            "zone_safety_verified",
            "pvp_event_inferred",
            "game_execution",
            "live",
        ] {
            assert_eq!(value[field], false, "{field}");
        }
    }

    #[test]
    fn player_pvp_flags_all_states_and_nonempty_predicates() {
        for mask in 0..8 {
            let raw = section(mask);
            let saved = snapshot(Some(raw.clone()));
            saved.validate().unwrap();
            let observed = observation(&raw);
            assert_boundary(&observed);
            for field in FLAGS {
                assert_eq!(observed["reported_flags"][field], raw[field]);
            }
            for code in 1..27 {
                let mut code = code;
                let mut predicates = serde_json::Map::new();
                let mut expected = true;
                for field in FLAGS {
                    let digit = code % 3;
                    code /= 3;
                    if digit != 0 {
                        let value = digit == 2;
                        predicates.insert(field.into(), json!(value));
                        expected &= raw[field] == value;
                    }
                }
                let filter: PlayerPvpFlagFilter =
                    serde_json::from_value(Value::Object(predicates.clone())).unwrap();
                filter.validate().unwrap();
                let matched = filter.observation_match(&saved);
                assert_eq!(
                    matched.is_some(),
                    expected,
                    "mask={mask} predicates={predicates:?}"
                );
                if let Some(matched) = matched {
                    assert_boundary(&matched);
                    assert_eq!(matched["filter"], Value::Object(predicates));
                }
            }
        }
    }

    #[test]
    fn player_pvp_flags_all_64_comparisons() {
        for before_mask in 0..8 {
            for after_mask in 0..8 {
                let before = section(before_mask);
                let after = section(after_mask);
                let result = compare(Some(&before), Some(&after)).unwrap();
                assert_boundary(&result);
                assert_eq!(result["before"], observation(&before));
                assert_eq!(result["after"], observation(&after));
                assert_eq!(
                    result["changed_flag_count"],
                    (before_mask ^ after_mask).count_ones()
                );
                let changes = result["flag_changes"].as_object().unwrap();
                for field in FLAGS {
                    if before[field] == after[field] {
                        assert!(!changes.contains_key(field));
                    } else {
                        assert_eq!(
                            changes[field],
                            json!({"before":before[field],"after":after[field]})
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn player_pvp_flags_missing_sides_and_absent_filter_observations() {
        let raw = section(7);
        for (before, after) in [(None, None), (Some(&raw), None), (None, Some(&raw))] {
            let result = compare(before, after).unwrap();
            assert_boundary(&result);
            assert_eq!(result["before"], json!(before.map(observation)));
            assert_eq!(result["after"], json!(after.map(observation)));
            assert!(result["flag_changes"].is_null());
            assert!(result["changed_flag_count"].is_null());
        }
        let filter: PlayerPvpFlagFilter = serde_json::from_value(json!({"is_pvp":false})).unwrap();
        assert!(filter.observation_match(&snapshot(None)).is_none());
        for family in [Family::Classic, Family::Retail] {
            let mut saved = snapshot(Some(section(0)));
            saved.client.family = family;
            assert!(saved.validate().is_err());
            assert!(filter.observation_match(&saved).is_none());
        }
    }

    #[test]
    fn player_pvp_flags_strict_section_and_filter_validation() {
        for field in ["coverage", "is_pvp", "is_free_for_all", "is_sanctuary"] {
            let mut missing = section(0);
            missing.as_object_mut().unwrap().remove(field);
            assert!(snapshot(Some(missing.clone())).validate().is_err());
            assert!(compare(Some(&missing), None).is_err());
            let mut null = section(0);
            null[field] = Value::Null;
            assert!(snapshot(Some(null)).validate().is_err());
        }
        for field in FLAGS {
            for bad in [json!(0), json!("false"), json!([]), json!({})] {
                let mut raw = section(0);
                raw[field] = bad.clone();
                assert!(snapshot(Some(raw.clone())).validate().is_err());
                assert!(compare(None, Some(&raw)).is_err());
                assert!(serde_json::from_value::<PlayerPvpFlagFilter>(json!({field:bad})).is_err());
            }
        }
        for invalid in [
            json!({}),
            json!({"is_pvp":null}),
            json!({"is_pvp":null,"is_free_for_all":null,"is_sanctuary":null}),
        ] {
            assert!(
                serde_json::from_value::<PlayerPvpFlagFilter>(invalid)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        assert!(
            serde_json::from_value::<PlayerPvpFlagFilter>(json!({"is_pvp":false,"unknown":true}))
                .is_err()
        );
        let filter: PlayerPvpFlagFilter =
            serde_json::from_value(json!({"is_pvp":false,"is_sanctuary":null})).unwrap();
        filter.validate().unwrap();
        assert_eq!(json!(filter), json!({"is_pvp":false}));
        let mut extra = section(0);
        extra["unknown"] = json!(false);
        assert!(snapshot(Some(extra)).validate().is_err());
        let mut bad_coverage = section(0);
        bad_coverage["coverage"] = json!("reported_player_life_flags");
        assert!(snapshot(Some(bad_coverage)).validate().is_err());
    }
}
