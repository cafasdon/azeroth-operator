//! Owner-local raw life-flag analysis, without life classification or actions.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const FLAGS: [&str; 4] = [
    "is_dead",
    "is_ghost",
    "is_dead_or_ghost",
    "is_feigning_death",
];

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerLifeStateFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dead: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_ghost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dead_or_ghost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_feigning_death: Option<bool>,
}

impl PlayerLifeStateFilter {
    fn predicates(&self) -> [(&'static str, Option<bool>); 4] {
        [
            ("is_dead", self.is_dead),
            ("is_ghost", self.is_ghost),
            ("is_dead_or_ghost", self.is_dead_or_ghost),
            ("is_feigning_death", self.is_feigning_death),
        ]
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.predicates().iter().any(|(_, value)| value.is_some()),
            "Nonempty player life-state filter required"
        );
        Ok(())
    }
    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        self.validate().ok()?;
        let section = snapshot.sections.get("player_life_state")?;
        crate::schema::validate("player_life_state", section).ok()?;
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
    json!({"basis":"captured_player_life_flags_only","current_life_state_verified":false,
        "character_continuity_verified":false,"resurrection_eligibility_verified":false,
        "life_event_inferred":false,"game_execution":false,"live":false})
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
        crate::schema::validate("player_life_state", section)?;
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
