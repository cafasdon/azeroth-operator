//! Saved purchased-rank comparisons only; no talent import or game execution.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAP: &[u8] = include_bytes!("../clients/forever/talent-map.json");
pub const MAP_SHA256: &str = "33dc07ad7ba838a4bb6f95d825df44235b0e1a00e92eb3dfdb9c68d8d96c18b3";

/// Prepare only when every pinned definition has an explicit captured rank.
pub fn prepare(snapshot: &Snapshot) -> Result<Value> {
    snapshot.validate()?;
    ensure!(
        snapshot.client.family == Family::Forever,
        "Forever snapshot required"
    );
    ensure!(
        format!("{:x}", Sha256::digest(MAP)) == MAP_SHA256,
        "Invalid pinned talent map"
    );
    let map: Value = serde_json::from_slice(MAP)?;
    let mut report = json!({"status":"unavailable","basis":"queried_config_tree_entries_purchased_ranks",
        "talent_map_sha256":MAP_SHA256,"requires_player_review":true,
        "allocation_match_verified":false,"native_tree_coverage_verified":false,
        "simulation_model_verified":false,"build_legality_verified":false,
        "native_import_generated":false,"game_execution":false,"live":false});
    let class = snapshot
        .sections
        .get("stats")
        .and_then(|stats| stats["class"].as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let Some(source) = map["classes"].get(&class) else {
        report["reason"] = json!("captured_class_unmapped");
        return Ok(report);
    };
    let Some(talents) = snapshot.sections.get("talents") else {
        report["reason"] = json!("definition_allocation_not_captured");
        return Ok(report);
    };
    let Some(allocation) = talents.get("definition_allocation") else {
        report["reason"] = json!("definition_allocation_not_captured");
        return Ok(report);
    };
    let definitions = allocation["definitions"].as_object().unwrap();
    let pinned = source["talents"].as_array().unwrap();
    let expected: BTreeSet<_> = pinned
        .iter()
        .map(|row| row["definition_id"].as_u64().unwrap())
        .collect();
    let missing: Vec<_> = expected
        .iter()
        .filter(|id| !definitions.contains_key(&id.to_string()))
        .copied()
        .collect();
    let extra: Vec<_> = definitions
        .keys()
        .map(|id| id.parse::<u64>().unwrap())
        .filter(|id| !expected.contains(id))
        .collect();
    let limits: Vec<_> = pinned
        .iter()
        .filter_map(|row| {
            let id = row["definition_id"].as_u64().unwrap();
            definitions
                .get(&id.to_string())
                .filter(|observed| observed["max_rank"] != row["max_rank"])
                .map(|_| id)
        })
        .collect();
    let staged = talents
        .get("staged_changes_observed")
        .and_then(Value::as_bool);
    report["captured_definition_count"] = json!(definitions.len());
    report["pinned_definition_count"] = json!(expected.len());
    report["missing_definition_ids"] = json!(missing);
    report["unmapped_definition_count"] = json!(extra.len());
    report["unmapped_definition_ids"] = json!(extra.iter().take(64).collect::<Vec<_>>());
    report["unmapped_definition_ids_truncated"] = json!(extra.len() > 64);
    report["rank_limit_disagreement_ids"] = json!(limits);
    report["staged_changes_observed"] = json!(staged);
    if !missing.is_empty() || !extra.is_empty() || !limits.is_empty() {
        report["reason"] = json!("pinned_definition_mismatch");
        return Ok(report);
    }
    if staged != Some(false) {
        report["reason"] = json!("staged_changes_or_unknown");
        return Ok(report);
    }
    let mut offset = 0;
    let mut strings = Vec::new();
    for size in source["tree_sizes"].as_array().unwrap() {
        let size = size.as_u64().unwrap() as usize;
        let digits = pinned[offset..offset + size]
            .iter()
            .map(|row| {
                definitions[&row["definition_id"].as_u64().unwrap().to_string()]["purchased_rank"]
                    .as_u64()
                    .unwrap()
                    .to_string()
            })
            .collect::<String>();
        strings.push(digits);
        offset += size;
    }
    report["status"] = json!("prepared_for_snapshot_review");
    report["simulator_talent_string"] = json!(strings.join("-"));
    Ok(report)
}

/// Compare only observed definitions; absence never proves an unspent talent.
pub fn review(snapshot: &Snapshot, talent_string: &str) -> Result<Value> {
    snapshot.validate()?;
    ensure!(
        snapshot.client.family == Family::Forever,
        "Forever snapshot required"
    );
    ensure!(
        talent_string.len() <= 256
            && talent_string
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-'),
        "Invalid simulator talent string"
    );
    let parts: Vec<_> = talent_string.split('-').collect();
    ensure!(parts.len() <= 3, "Too many talent trees");
    ensure!(
        format!("{:x}", Sha256::digest(MAP)) == MAP_SHA256,
        "Invalid pinned talent map"
    );
    let map: Value = serde_json::from_slice(MAP)?;
    let mut report = json!({"status":"unverified","basis":"captured_purchased_ranks_and_pinned_definitions",
        "talent_map_sha256":MAP_SHA256,"allocation_match_verified":false,
        "native_tree_coverage_verified":false,"simulation_model_verified":false,
        "compared_definition_count":0,"unknown_node_count":0,"rank_limit_disagreement_count":0,"conflicts":[]});
    let class = snapshot
        .sections
        .get("stats")
        .and_then(|stats| stats["class"].as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let Some(source) = map["classes"].get(&class) else {
        report["reason"] = json!("captured_class_unmapped");
        return Ok(report);
    };
    let mut ranks = Vec::new();
    for (index, size) in source["tree_sizes"].as_array().unwrap().iter().enumerate() {
        let size = size.as_u64().unwrap() as usize;
        let part = parts.get(index).copied().unwrap_or("");
        ensure!(part.len() <= size, "Talent string exceeds pinned tree");
        ranks.extend(part.bytes().map(|byte| u64::from(byte - b'0')));
        ranks.extend(std::iter::repeat_n(0, size - part.len()));
    }
    let Some(observed) = snapshot
        .sections
        .get("talents")
        .filter(|talents| talents.get("observed_tree_ids").is_some())
    else {
        return Ok(report);
    };
    let mapped: BTreeMap<_, _> = source["talents"]
        .as_array()
        .unwrap()
        .iter()
        .zip(ranks)
        .map(|(talent, rank)| {
            let maximum = talent["max_rank"].as_u64().unwrap();
            (
                talent["definition_id"].as_u64().unwrap(),
                (rank.min(maximum), maximum),
            )
        })
        .collect();
    let mut definitions = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut unknown = 0;
    for node in observed["nodes"].as_object().unwrap().values() {
        let Some(definition) = node["definition_id"].as_u64() else {
            unknown += 1;
            continue;
        };
        ensure!(seen.insert(definition), "Ambiguous captured definition");
        definitions.insert(definition, node);
    }
    let mut compared = 0;
    let mut limit_disagreements = 0;
    let mut conflicts = Vec::new();
    for (definition, node) in definitions {
        let Some((rank, maximum)) = mapped.get(&definition) else {
            unknown += 1;
            continue;
        };
        if node["max_rank"].as_u64() != Some(*maximum) {
            limit_disagreements += 1;
            continue;
        }
        compared += 1;
        if node["purchased_rank"].as_u64() != Some(*rank) {
            conflicts.push(
                json!({"definition_id":definition,"captured_purchased_rank":node["purchased_rank"],
                "template_effective_rank":rank}),
            );
        }
    }
    let staged = observed
        .get("staged_changes_observed")
        .and_then(Value::as_bool);
    report["status"] = json!(match staged {
        Some(true) => "staged_changes",
        Some(false) if !conflicts.is_empty() => "mismatch",
        Some(false) if compared > 0 => "observed_ranks_match",
        _ => "unverified",
    });
    report["compared_definition_count"] = json!(compared);
    report["unknown_node_count"] = json!(unknown);
    report["rank_limit_disagreement_count"] = json!(limit_disagreements);
    report["staged_changes_observed"] = json!(staged);
    report["conflicts"] = json!(conflicts);
    Ok(report)
}
