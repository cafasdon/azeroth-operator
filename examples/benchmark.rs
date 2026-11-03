//! Repeatable synthetic baseline. No real character data.
use azeroth_operator::{Family, Snapshot, Store};
use serde_json::json;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store {
        root: directory.path().into(),
    };
    let mut ids = Vec::new();
    for n in 0..200 {
        let items: serde_json::Map<String, serde_json::Value> = (1..=180)
            .map(|slot| {
                (
                    slot.to_string(),
                    json!({"item_id":1000+slot,"count":20,"link":"synthetic-item-link"}),
                )
            })
            .collect();
        let snapshot: Snapshot = serde_json::from_value(
            json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":format!("Fixture{n}"),"realm":"Test"},"sections":{"equipment":{},"bags":{"0":items}}}),
        )?;
        ids.push(store.ingest(&snapshot)?);
    }
    // Warm file cache; this is explicitly a warm-cache measurement.
    store.list(Family::Retail, 0, 20)?;
    let mut listing = Vec::new();
    for _ in 0..20 {
        let now = Instant::now();
        store.list(Family::Retail, 0, 20)?;
        listing.push(now.elapsed().as_secs_f64() * 1000.0);
    }
    let mut inspect = Vec::new();
    for n in 0..200 {
        let now = Instant::now();
        store.inspect(&ids[n % ids.len()], None)?;
        inspect.push(now.elapsed().as_secs_f64() * 1000.0);
    }
    listing.sort_by(f64::total_cmp);
    inspect.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"profile":"release","cache":"warm","snapshots":200,"items_per_snapshot":180,"list_runs":20,"inspect_runs":200,"list_ms":{"median":listing[10],"p95":listing[18]},"inspect_ms":{"median":inspect[100],"p95":inspect[189]}})
        )?
    );
    Ok(())
}
