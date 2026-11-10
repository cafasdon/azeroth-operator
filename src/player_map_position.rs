//! Private saved map diagram coordinates. Geometry is not a traversable route.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub map_id: std::num::NonZeroU32,
    pub x: f64,
    pub y: f64,
}
impl Target {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.x.is_finite()
                && self.y.is_finite()
                && (0.0..=1.0).contains(&self.x)
                && (0.0..=1.0).contains(&self.y),
            "Invalid map target"
        );
        Ok(())
    }
}

pub fn observation(snapshot: &Snapshot) -> Value {
    json!({"capture_status":snapshot.section_capture_status("player_map_position"),
        "position":snapshot.sections.get("player_map_position"),"basis":"saved_map_diagram_coordinates",
        "current_position_verified":false,"active_character_verified":false,
        "world_distance_verified":false,"travel_route_verified":false,"game_execution":false,"live":false})
}

pub fn preview(snapshot: &Snapshot, target: &Target) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a saved Forever position"
    );
    target.validate()?;
    let mut report = observation(snapshot);
    report["target"] = json!(target);
    report["diagram_delta"] = Value::Null;
    report["status"] = json!(snapshot.section_capture_status("player_map_position"));
    if let Some(position) = snapshot.sections.get("player_map_position") {
        if position["map_id"].as_u64() != Some(u64::from(target.map_id.get())) {
            report["status"] = json!("different_map");
        } else {
            let x = position["x"]
                .as_f64()
                .ok_or_else(|| anyhow::anyhow!("Position unavailable"))?;
            let y = position["y"]
                .as_f64()
                .ok_or_else(|| anyhow::anyhow!("Position unavailable"))?;
            let dx = target.x - x;
            let dy = target.y - y;
            report["status"] = json!("same_map_diagram_only");
            report["diagram_delta"] = json!({"x":dx,"y":dy,"euclidean_separation":dx.hypot(dy),
                "units":"normalised_map_axes_not_yards"});
        }
    }
    Ok(report)
}
