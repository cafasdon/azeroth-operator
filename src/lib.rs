//! Data-only client contracts and snapshot storage. No game execution.
pub mod action_operation;
pub mod analysis;
pub mod bag_operation;
pub mod binding_operation;
pub mod equipment_operation;
pub mod equipment_set_operation;
pub mod forever_catalogue;
pub mod import;
pub mod macro_operation;
pub mod planning;
mod schema;
pub mod sync;
pub mod talent_operation;
pub mod upload;
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_BYTES: u64 = 4 * 1024 * 1024;
const SECTIONS: &[&str] = &[
    "equipment",
    "equipment_sets",
    "bags",
    "character_bank",
    "account_bank",
    "selected_recipe",
    "tracked_recipes",
    "reputation",
    "currency_balances",
    "wallet",
    "experience",
    "spellbook",
    "flight_map",
    "durability_alerts",
    "repair_quote",
    "tracked_achievements",
    "talents",
    "stats",
    "professions",
    "quests",
    "currencies",
    "achievements",
    "macros",
    "bindings",
    "action_bars",
    "operation_receipts",
    "binding_state",
    "action_bar_observation",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Retail,
    Classic,
    Forever,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientIdentity {
    pub family: Family,
    pub variant: String,
    pub build: String,
    pub interface: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub name: String,
    pub realm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Addon,
    OfficialApi,
    Fixture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub source: Source,
    pub captured_at: DateTime<Utc>,
    pub client: ClientIdentity,
    pub character: Character,
    pub sections: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub unavailable: BTreeMap<String, String>,
}

impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unsupported schema version");
        ensure!(self.client.interface > 0, "Missing interface version");
        for text in [
            &self.client.variant,
            &self.client.build,
            &self.character.name,
            &self.character.realm,
        ] {
            ensure!(
                !text.trim().is_empty() && text.len() <= 128,
                "Invalid identity field"
            );
        }
        ensure!(
            self.captured_at <= Utc::now(),
            "Capture timestamp is in the future"
        );
        for (name, value) in &self.sections {
            ensure!(
                !matches!(
                    name.as_str(),
                    "selected_recipe"
                        | "tracked_recipes"
                        | "reputation"
                        | "currency_balances"
                        | "tracked_achievements"
                        | "wallet"
                        | "experience"
                        | "spellbook"
                        | "flight_map"
                        | "durability_alerts"
                        | "repair_quote"
                ) || self.client.family == Family::Forever,
                "Observation requires Forever"
            );
            schema::validate(name, value)?;
            ensure!(SECTIONS.contains(&name.as_str()), "Unknown section");
            ensure!(
                value.is_array() || value.is_object(),
                "Section must be an array or object"
            );
        }
        if self.sections.contains_key("character_bank")
            || self.sections.contains_key("account_bank")
        {
            ensure!(
                self.client.family == Family::Forever,
                "Bank snapshot requires Forever"
            );
            let mut containers = std::collections::BTreeSet::new();
            for name in ["bags", "character_bank", "account_bank"] {
                if let Some(bags) = self.sections.get(name).and_then(Value::as_object) {
                    for bag in bags.keys() {
                        ensure!(
                            containers.insert(bag),
                            "Container appears in multiple storage sections"
                        );
                    }
                }
            }
        }
        for (name, reason) in &self.unavailable {
            ensure!(
                SECTIONS.contains(&name.as_str()) && !self.sections.contains_key(name),
                "Invalid unavailable section"
            );
            ensure!(
                !reason.trim().is_empty() && reason.len() <= 256,
                "Invalid unavailable reason"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
}

#[derive(Default)]
pub struct RecipeCaptureFilters {
    pub recipe_id: Option<std::num::NonZeroU32>,
    pub output_item_id: Option<std::num::NonZeroU32>,
    pub profession_id: Option<std::num::NonZeroU32>,
}

impl Store {
    pub fn list(&self, family: Family, offset: usize, limit: usize) -> Result<Value> {
        self.list_matching(family, None, offset, limit)
    }

    pub fn list_matching(
        &self,
        family: Family,
        character: Option<&Character>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_matching_recipe_output(family, character, None, offset, limit)
    }

    pub fn list_matching_recipe_output(
        &self,
        family: Family,
        character: Option<&Character>,
        recipe_output_item_id: Option<std::num::NonZeroU32>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_matching_recipes(
            family,
            character,
            RecipeCaptureFilters {
                output_item_id: recipe_output_item_id,
                ..Default::default()
            },
            offset,
            limit,
        )
    }

    pub fn list_matching_recipes(
        &self,
        family: Family,
        character: Option<&Character>,
        filters: RecipeCaptureFilters,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        let filter_recipes = filters.recipe_id.is_some()
            || filters.output_item_id.is_some()
            || filters.profession_id.is_some();
        ensure!(
            !filter_recipes || family == Family::Forever,
            "Recipe filtering requires Forever"
        );
        ensure!((1..=100).contains(&limit), "Limit must be 1..100");
        if let Some(character) = character {
            ensure!(
                [&character.name, &character.realm]
                    .into_iter()
                    .all(|value| !value.trim().is_empty() && value.len() <= 128),
                "Invalid character identity"
            );
        }
        if !self.root.exists() {
            return Ok(json!({"snapshots":[],"has_more":false,"invalid_count":0}));
        }
        // Resolve once per scan; every file still gets containment/hash/schema checks.
        let root = self.root.canonicalize()?;
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&root)? {
            let path = entry?.path();
            if path.extension().is_some_and(|x| x == "json") {
                paths.push(path);
            }
            ensure!(
                paths.len() <= 10000,
                "Store scan limit reached; archive older snapshots"
            );
        }
        paths.sort();
        let mut rows = Vec::new();
        let mut invalid = 0;
        for path in paths {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                invalid += 1;
                continue;
            };
            match Self::read_in_root(&root, id) {
                Ok(s)
                    if s.client.family == family
                        && filters.recipe_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["recipe_id"].as_u64() == Some(u64::from(id.get()))
                            })
                        })
                        && filters.output_item_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["output_item_id"].as_u64() == Some(u64::from(id.get()))
                            })
                        })
                        && filters.profession_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["profession"]["profession_id"].as_u64()
                                    == Some(u64::from(id.get()))
                            })
                        })
                        && character.is_none_or(|target| {
                            s.character.name.to_lowercase() == target.name.to_lowercase()
                                && s.character.realm.to_lowercase() == target.realm.to_lowercase()
                        }) =>
                {
                    // Retain metadata only: listing must not hold every inventory payload.
                    let mut summary = json!({"snapshot_id":id,"character":s.character,"client":s.client,"captured_at":s.captured_at,"source":s.source});
                    if filter_recipes && let Some(recipe) = s.sections.get("selected_recipe") {
                        summary["selected_recipe"] = json!({
                            "recipe_id":recipe["recipe_id"],"learned":recipe["learned"],
                            "output_item_id":recipe["output_item_id"]
                        });
                        if filters.profession_id.is_some() {
                            summary["selected_recipe"]["profession_id"] =
                                recipe["profession"]["profession_id"].clone();
                        }
                    }
                    rows.push((s.captured_at, id.to_owned(), summary));
                }
                Ok(_) => (),
                Err(_) => invalid += 1,
            }
        }
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let has_more = rows.len().saturating_sub(offset) > limit;
        let snapshots: Vec<_> = rows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(_, _, summary)| summary)
            .collect();
        Ok(json!({"snapshots":snapshots,"has_more":has_more,"invalid_count":invalid}))
    }
    pub fn ingest(&self, snapshot: &Snapshot) -> Result<String> {
        snapshot.validate()?;
        let raw = serde_json::to_vec(snapshot)?;
        ensure!(raw.len() as u64 <= MAX_BYTES, "Snapshot exceeds size limit");
        let id = format!("{:x}", Sha256::digest(&raw));
        std::fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{id}.json"));
        if path.exists() {
            self.read(&id)?;
            return Ok(id);
        }
        let mut pending = tempfile::NamedTempFile::new_in(&self.root)?;
        pending.write_all(&raw)?;
        pending.as_file().sync_all()?;
        match pending.persist_noclobber(&path) {
            Ok(_) => (),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.read(&id)?;
            }
            Err(e) => return Err(e.error.into()),
        }
        Ok(id)
    }

    pub fn read(&self, id: &str) -> Result<Snapshot> {
        let root = self.root.canonicalize()?;
        Self::read_in_root(&root, id)
    }

    fn read_in_root(root: &Path, id: &str) -> Result<Snapshot> {
        ensure!(
            id.len() == 64
                && id
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "Invalid snapshot id"
        );
        let path = root.join(format!("{id}.json")).canonicalize()?;
        ensure!(path.parent() == Some(root), "Snapshot path escaped store");
        let mut raw = Vec::new();
        File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut raw)?;
        ensure!(raw.len() as u64 <= MAX_BYTES, "Snapshot exceeds size limit");
        ensure!(
            format!("{:x}", Sha256::digest(&raw)) == id,
            "Snapshot content hash mismatch"
        );
        let snapshot: Snapshot = serde_json::from_slice(&raw)?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn inspect(&self, id: &str, section: Option<&str>) -> Result<Value> {
        let s = self.read(id)?;
        let mut result = json!({"snapshot_id": id, "client": s.client, "character": s.character,
            "source": s.source, "captured_at": s.captured_at, "age_seconds": (Utc::now()-s.captured_at).num_seconds(), "live": false});
        if let Some(name) = section {
            ensure!(SECTIONS.contains(&name), "Unknown section");
            result["available"] = json!(s.sections.contains_key(name));
            result["data"] = s.sections.get(name).cloned().unwrap_or(Value::Null);
            result["unavailable_reason"] = json!(s.unavailable.get(name));
        } else {
            result["available_sections"] = json!(s.sections.keys().collect::<Vec<_>>());
            result["unavailable"] = json!(s.unavailable);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}})).unwrap()
    }
    #[test]
    fn roundtrip_and_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let id = store.ingest(&fixture()).unwrap();
        assert_eq!(id, store.ingest(&fixture()).unwrap());
        assert_eq!(store.inspect(&id, None).unwrap()["live"], false);
        assert!(store.read("../secret").is_err());
        std::fs::write(dir.path().join(format!("{id}.json")), "{}").unwrap();
        assert!(store.read(&id).is_err());
    }
    #[test]
    fn invalid_envelope() {
        let mut s = fixture();
        s.client.interface = 0;
        assert!(s.validate().is_err());
        let mut s = fixture();
        s.sections.insert("execute".into(), json!({}));
        assert!(s.validate().is_err());
        let mut s = fixture();
        s.captured_at = Utc::now() + chrono::Duration::days(1);
        assert!(s.validate().is_err());
        let mut v = serde_json::to_value(fixture()).unwrap();
        v["client"]["family"] = json!("unknown");
        assert!(serde_json::from_value::<Snapshot>(v).is_err());
    }
    #[test]
    fn concurrent_publication() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                std::thread::spawn(move || store.ingest(&fixture()).unwrap())
            })
            .collect();
        let ids: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert!(store.read(&ids[0]).is_ok());
    }

    #[test]
    fn listing_preserves_validation_family_order_and_pages() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let mut expected = Vec::new();
        for name in ["A", "B", "C"] {
            let mut snapshot = fixture();
            snapshot.character.name = name.into();
            expected.push(store.ingest(&snapshot).unwrap());
        }
        expected.sort(); // Equal capture times use ascending content ID.
        let mut other = fixture();
        other.client.family = Family::Forever;
        store.ingest(&other).unwrap();
        let mut invalid = serde_json::to_value(fixture()).unwrap();
        invalid["sections"]["equipment"] = json!({"16":{"item_id":0}});
        let raw = serde_json::to_vec(&invalid).unwrap();
        let id = format!("{:x}", Sha256::digest(&raw));
        std::fs::write(dir.path().join(format!("{id}.json")), raw).unwrap();
        std::fs::write(dir.path().join(format!("{}.json", "a".repeat(64))), "{}").unwrap();
        let first = store.list(Family::Retail, 0, 2).unwrap();
        let last = store.list(Family::Retail, 2, 2).unwrap();
        assert_eq!(first["invalid_count"], 2);
        assert_eq!(first["has_more"], true);
        assert_eq!(last["has_more"], false);
        assert_eq!(first["snapshots"][0]["snapshot_id"], expected[0]);
        assert_eq!(first["snapshots"][1]["snapshot_id"], expected[1]);
        assert_eq!(last["snapshots"][0]["snapshot_id"], expected[2]);
        assert_eq!(
            store.list(Family::Forever, 0, 2).unwrap()["snapshots"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store.list(Family::Retail, usize::MAX, 2).unwrap()["snapshots"],
            json!([])
        );
        std::fs::write(dir.path().join(format!("{}.json", expected[0])), "{}").unwrap();
        let fresh = store.list(Family::Retail, 0, 100).unwrap();
        assert_eq!(fresh["invalid_count"], 3);
        assert_eq!(fresh["snapshots"].as_array().unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn listing_rejects_hash_valid_symlink_outside_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("store");
        std::fs::create_dir(&root).unwrap();
        let raw = serde_json::to_vec(&fixture()).unwrap();
        let id = format!("{:x}", Sha256::digest(&raw));
        let outside = dir.path().join("outside.json");
        std::fs::write(&outside, raw).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(format!("{id}.json"))).unwrap();
        let store = Store { root };
        assert!(store.read(&id).is_err());
        let listing = store.list(Family::Retail, 0, 20).unwrap();
        assert_eq!(listing["snapshots"], json!([]));
        assert_eq!(listing["invalid_count"], 1);
    }
}
