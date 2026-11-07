use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const PIN: &str = include_str!("../clients/forever/catalogue.lock.json");

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Items,
    Enchants,
    RandomSuffixes,
    Zones,
    Npcs,
    Factions,
    Encounters,
    Consumables,
    SpellEffects,
}

impl Section {
    fn fields(&self) -> (&'static str, &'static str) {
        match self {
            Self::Items => ("items", "id"),
            Self::Enchants => ("enchants", "effectId"),
            Self::RandomSuffixes => ("randomSuffixes", "id"),
            Self::Zones => ("zones", "id"),
            Self::Npcs => ("npcs", "id"),
            Self::Factions => ("factions", "id"),
            Self::Encounters => ("encounters", "path"),
            Self::Consumables => ("consumables", "id"),
            Self::SpellEffects => ("spellEffects", "id"),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub section: Section,
    /// Exact numeric ID (enchant effect ID for enchants), encounter path, or name substring.
    /// Omit to get the section count.
    pub key: Option<String>,
}

pub fn lookup(path: &Path, query: &Query) -> Result<Value> {
    let provenance: Value = serde_json::from_str(PIN)?;
    ensure!(
        path.metadata()?.is_file(),
        "Catalogue must be a regular file"
    );
    let mut data = Vec::new();
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() as u64 <= MAX_BYTES, "Catalogue too large");
    ensure!(
        provenance["sha256"] == format!("{:x}", Sha256::digest(&data)),
        "Catalogue checksum mismatch"
    );
    let db: Value = serde_json::from_slice(&data)?;
    let (section, identity) = query.section.fields();
    let rows = db[section]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue section"))?;
    let mut records = Vec::new();
    let mut has_more = false;
    if let Some(key) = &query.key {
        let key = key.trim();
        ensure!(!key.is_empty() && key.len() <= 160, "Invalid catalogue key");
        let id = key.parse::<u32>().ok();
        let name = key.to_lowercase();
        for row in rows {
            let matches = if identity == "path" {
                row[identity].as_str() == Some(key)
            } else if let Some(id) = id {
                row[identity].as_u64() == Some(u64::from(id))
            } else {
                row["name"]
                    .as_str()
                    .is_some_and(|s| s.to_lowercase().contains(&name))
            };
            if matches {
                if records.len() == 20 {
                    has_more = true;
                    break;
                }
                records.push(row);
            }
        }
    }
    Ok(
        json!({"family":"forever","source":provenance,"section":section,
        "section_count":rows.len(),"records":records,"has_more":has_more,
        "acquisition_verified":false}),
    )
}
