//! Saved input profiles, not permission, live readiness or workflow execution.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

struct Workflow {
    id: &'static str,
    tool: &'static str,
    section: Option<&'static str>,
    additional_arguments: bool,
    required: &'static [&'static str],
    helpful: &'static [&'static str],
    outside_checks: &'static [&'static str],
}

const WORKFLOWS: &[Workflow] = &[
    Workflow {
        id: "equipment_selection",
        tool: "review_equipment_selection",
        section: None,
        additional_arguments: true,
        required: &["equipment"],
        helpful: &["bags", "stats", "weapon_enchants"],
        outside_checks: &["selection_arguments", "native_equip_legality"],
    },
    Workflow {
        id: "owned_gear_pool",
        tool: "review_forever_owned_gear",
        section: None,
        additional_arguments: false,
        required: &["equipment", "bags", "character_bank", "account_bank"],
        helpful: &[
            "stats",
            "skill_lines",
            "talents",
            "spellbook",
            "profession_spells",
        ],
        outside_checks: &[
            "catalogue_configuration",
            "exact_variant_resolution",
            "complete_ownership",
            "native_equip_legality",
            "simulator_model_and_search_coverage",
        ],
    },
    Workflow {
        id: "equipment_condition_and_repair",
        tool: "inspect_snapshot",
        section: Some("equipment_durability"),
        additional_arguments: true,
        required: &["equipment_durability", "repair_quote"],
        helpful: &["wallet", "durability_alerts"],
        outside_checks: &["review_arguments", "current_repair_availability"],
    },
    Workflow {
        id: "bag_restock",
        tool: "inspect_snapshot",
        section: Some("bags"),
        additional_arguments: true,
        required: &["bags", "bag_capacity"],
        helpful: &["character_bank", "account_bank", "merchant_stock", "wallet"],
        outside_checks: &["target_quantities", "current_storage_access"],
    },
    Workflow {
        id: "recipe_material_search",
        tool: "search_forever_recipes",
        section: None,
        additional_arguments: true,
        required: &["profession_recipe_materials"],
        helpful: &[
            "profession_recipe_view",
            "profession_spells",
            "tracked_recipes",
            "bags",
        ],
        outside_checks: &["search_arguments", "native_recipe_coverage"],
    },
    Workflow {
        id: "recipe_chain_review",
        tool: "review_forever_recipe",
        section: None,
        additional_arguments: true,
        required: &["selected_recipe", "profession_recipe_materials"],
        helpful: &[
            "bags",
            "character_bank",
            "account_bank",
            "profession_recipe_view",
            "profession_spells",
            "recipe_item_sources",
            "merchant_stock",
            "wallet",
            "tracked_recipes",
        ],
        outside_checks: &[
            "chain_arguments",
            "recipe_choice_resolution",
            "current_crafting_eligibility",
        ],
    },
    Workflow {
        id: "skill_target_review",
        tool: "inspect_snapshot",
        section: Some("skill_lines"),
        additional_arguments: true,
        required: &["skill_lines"],
        helpful: &["trainer_services", "wallet"],
        outside_checks: &["target_rank", "current_training_eligibility"],
    },
    Workflow {
        id: "quest_progress",
        tool: "inspect_snapshot",
        section: Some("quests"),
        additional_arguments: false,
        required: &["quests"],
        helpful: &["stats", "experience"],
        outside_checks: &["current_quest_state"],
    },
    Workflow {
        id: "quest_turn_in_rewards",
        tool: "inspect_snapshot",
        section: Some("quest_giver_references"),
        additional_arguments: true,
        required: &["quests", "quest_giver_references"],
        helpful: &["wallet", "experience", "currency_balances", "reputation"],
        outside_checks: &[
            "review_arguments",
            "native_provider_identity",
            "finisher_coverage",
            "current_turn_in_eligibility",
        ],
    },
    Workflow {
        id: "quest_reward_review",
        tool: "inspect_snapshot",
        section: Some("quests"),
        additional_arguments: true,
        required: &["quests"],
        helpful: &[
            "bags",
            "bag_capacity",
            "equipment",
            "experience",
            "wallet",
            "currency_balances",
            "reputation",
        ],
        outside_checks: &["reward_arguments", "choice_rewards", "native_bag_placement"],
    },
    Workflow {
        id: "flight_quotes",
        tool: "inspect_snapshot",
        section: Some("flight_map"),
        additional_arguments: true,
        required: &["flight_map"],
        helpful: &["wallet", "stats", "instance_context"],
        outside_checks: &["route_arguments", "current_flight_availability"],
    },
    Workflow {
        id: "merchant_quote_review",
        tool: "inspect_snapshot",
        section: Some("merchant_stock"),
        additional_arguments: true,
        required: &["merchant_stock"],
        helpful: &["wallet", "bags", "bag_capacity"],
        outside_checks: &["offer_quantities", "current_merchant_availability"],
    },
    Workflow {
        id: "trainer_lesson_quote",
        tool: "inspect_snapshot",
        section: Some("trainer_services"),
        additional_arguments: true,
        required: &["trainer_services"],
        helpful: &["wallet", "skill_lines", "spellbook", "stats", "pet_care"],
        outside_checks: &["lesson_arguments", "current_training_eligibility"],
    },
    Workflow {
        id: "mail_attachment_review",
        tool: "inspect_snapshot",
        section: Some("mail_attachments"),
        additional_arguments: false,
        required: &["mail_attachments"],
        helpful: &["bags", "bag_capacity"],
        outside_checks: &[
            "native_mail_availability",
            "attachment_identity",
            "ownership_after_retrieval",
        ],
    },
    Workflow {
        id: "mail_cod_budget",
        tool: "inspect_snapshot",
        section: Some("mail_funds"),
        additional_arguments: true,
        required: &["mail_funds"],
        helpful: &["wallet"],
        outside_checks: &[
            "selected_quote_slots",
            "current_mail_identity",
            "payment_eligibility",
        ],
    },
    Workflow {
        id: "owned_auction_cache",
        tool: "inspect_snapshot",
        section: Some("owned_auctions"),
        additional_arguments: false,
        required: &["owned_auctions"],
        helpful: &["bags"],
        outside_checks: &["auction_cache_freshness", "ownership_after_transactions"],
    },
    Workflow {
        id: "pet_care_review",
        tool: "inspect_snapshot",
        section: Some("pet_care"),
        additional_arguments: true,
        required: &["pet_care", "pet_action_bar"],
        helpful: &["bags", "spellbook", "skill_lines"],
        outside_checks: &[
            "review_arguments",
            "current_pet_continuity",
            "current_pet_eligibility",
        ],
    },
    Workflow {
        id: "spell_resource_review",
        tool: "inspect_snapshot",
        section: Some("spellbook"),
        additional_arguments: true,
        required: &["spellbook", "player_resources"],
        helpful: &[
            "player_auras",
            "action_bar_observation",
            "bag_item_cooldowns",
        ],
        outside_checks: &[
            "spell_arguments",
            "current_casting_eligibility",
            "spellbook_coverage",
        ],
    },
    Workflow {
        id: "collection_views",
        tool: "character_summary",
        section: None,
        additional_arguments: false,
        required: &[
            "mount_collection",
            "pet_collection_view",
            "toy_collection_view",
        ],
        helpful: &["tracked_achievements", "reputation", "experience"],
        outside_checks: &["journal_filters", "complete_collection_coverage"],
    },
    Workflow {
        id: "progression_review",
        tool: "character_summary",
        section: None,
        additional_arguments: false,
        required: &["experience", "reputation", "currency_balances"],
        helpful: &["tracked_achievements", "player_pvp_progression"],
        outside_checks: &["complete_progression_coverage", "current_progression_state"],
    },
    Workflow {
        id: "instance_reset_schedule",
        tool: "character_summary",
        section: None,
        additional_arguments: true,
        required: &["saved_instances", "saved_world_bosses"],
        helpful: &["instance_context"],
        outside_checks: &["reference_time", "cache_age", "current_lockout_eligibility"],
    },
    Workflow {
        id: "pvp_observations",
        tool: "character_summary",
        section: None,
        additional_arguments: false,
        required: &[
            "player_pvp_flags",
            "player_pvp_progression",
            "battlefield_queue_slots",
        ],
        helpful: &["group_context", "instance_context", "player_life_state"],
        outside_checks: &["current_pvp_state", "zone_safety", "pvp_event_continuity"],
    },
    Workflow {
        id: "group_context_review",
        tool: "inspect_snapshot",
        section: Some("group_context"),
        additional_arguments: true,
        required: &["group_context"],
        helpful: &["instance_context", "saved_instances"],
        outside_checks: &[
            "review_arguments",
            "current_group_membership",
            "native_group_identity",
        ],
    },
    Workflow {
        id: "loot_and_trade_views",
        tool: "character_summary",
        section: None,
        additional_arguments: false,
        required: &["loot_window", "player_trade_offer"],
        helpful: &["bags", "bag_capacity", "equipment", "wallet"],
        outside_checks: &[
            "current_offer_identity",
            "acquisition_eligibility",
            "ownership_after_inventory_observation",
        ],
    },
];

/// Caller supplies a validated owner-local snapshot. Only section statuses are read.
pub fn report(snapshot: &Snapshot) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Workflow input report requires Forever"
    );
    let mut workflows = serde_json::Map::new();
    for workflow in WORKFLOWS {
        let statuses = |sections: &[&str]| {
            sections
                .iter()
                .map(|name| {
                    (
                        (*name).to_string(),
                        json!(snapshot.section_capture_status(name)),
                    )
                })
                .collect::<serde_json::Map<_, _>>()
        };
        let required = statuses(workflow.required);
        let helpful = statuses(workflow.helpful);
        let observed = required
            .values()
            .filter(|status| **status == "observed")
            .count();
        let unavailable = required
            .values()
            .filter(|status| **status == "reported_unavailable")
            .count();
        let missing = required
            .iter()
            .filter(|(_, status)| **status != "observed")
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        workflows.insert(workflow.id.into(),json!({
            "input_status":if observed==required.len() {"all_observed"} else if observed==0 {"none_observed"} else {"some_observed"},
            "all_required_inputs_observed":observed==required.len(),
            "required_sections":required,"helpful_sections":helpful,
            "required_status_counts":{"observed":observed,"reported_unavailable":unavailable,"not_recorded":workflow.required.len()-observed-unavailable},
            "missing_required_sections":missing,
            "entrypoint":{"tool":workflow.tool,"section":workflow.section,"additional_arguments_needed":workflow.additional_arguments,"catalogue_configuration_needed":workflow.tool == "review_forever_owned_gear"},
            "outside_snapshot_checks":workflow.outside_checks
        }));
    }
    Ok(
        json!({"basis":"saved_section_presence_only","target_basis":"explicit_saved_snapshot_id",
        "known_workflow_count":WORKFLOWS.len(),"workflows":workflows,
        "current_state_verified":false,"active_character_verified":false,"workflow_execution_verified":false,
        "complete_ownership_verified":false,"native_eligibility_verified":false,
        "complete_game_coverage_verified":false,"game_execution":false,"live":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn snapshot() -> Snapshot {
        let snapshot:Snapshot=serde_json::from_value(json!({"schema_version":1,"source":"fixture",
            "captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Fixture","realm":"Test"},"sections":{"equipment":{},"bags":{}},
            "unavailable":{"character_bank":"PRIVATE_REASON"}})).unwrap();
        snapshot.validate().unwrap();
        snapshot
    }
    #[test]
    fn definitions_use_unique_known_inputs_and_read_routes() {
        let mut ids = BTreeSet::new();
        for workflow in WORKFLOWS {
            assert!(ids.insert(workflow.id));
            assert!(!workflow.required.is_empty());
            let mut sections = BTreeSet::new();
            for section in workflow.required.iter().chain(workflow.helpful) {
                assert!(crate::SNAPSHOT_SECTIONS.contains(section));
                assert!(sections.insert(section));
            }
            assert!(matches!(
                workflow.tool,
                "inspect_snapshot"
                    | "character_summary"
                    | "review_equipment_selection"
                    | "review_forever_owned_gear"
                    | "search_forever_recipes"
                    | "review_forever_recipe"
            ));
            assert_eq!(
                workflow.tool == "inspect_snapshot",
                workflow.section.is_some()
            );
            if let Some(section) = workflow.section {
                assert!(workflow.required.contains(&section));
            }
            assert!(!workflow.outside_checks.is_empty());
        }
    }
    #[test]
    fn observed_zero_missing_and_unavailable_stay_distinct() {
        let snapshot = snapshot();
        let before = serde_json::to_vec(&snapshot).unwrap();
        let report = report(&snapshot).unwrap();
        let rows = &report["workflows"];
        assert_eq!(rows["equipment_selection"]["input_status"], "all_observed");
        let owned = &rows["owned_gear_pool"];
        assert_eq!(owned["input_status"], "some_observed");
        assert_eq!(
            owned["required_status_counts"],
            json!({"observed":2,"reported_unavailable":1,"not_recorded":1})
        );
        assert_eq!(
            owned["required_sections"]["character_bank"],
            "reported_unavailable"
        );
        assert_eq!(owned["required_sections"]["account_bank"], "not_recorded");
        assert_eq!(rows["mail_cod_budget"]["input_status"], "none_observed");
        assert!(!serde_json::to_string(&report).unwrap().contains("PRIVATE"));
        assert_eq!(serde_json::to_vec(&snapshot).unwrap(), before);
        for flag in [
            "current_state_verified",
            "active_character_verified",
            "workflow_execution_verified",
            "complete_ownership_verified",
            "native_eligibility_verified",
            "complete_game_coverage_verified",
            "game_execution",
            "live",
        ] {
            assert_eq!(report[flag], false);
        }
    }
    #[test]
    fn full_storage_presence_does_not_prove_ownership_or_execution() {
        let mut snapshot = snapshot();
        snapshot.unavailable.clear();
        snapshot.sections.insert("character_bank".into(), json!({}));
        snapshot.sections.insert("account_bank".into(), json!({}));
        snapshot.validate().unwrap();
        let complete = report(&snapshot).unwrap();
        assert_eq!(
            complete["workflows"]["owned_gear_pool"]["input_status"],
            "all_observed"
        );
        assert_eq!(complete["complete_ownership_verified"], false);
        snapshot.client.family = Family::Retail;
        assert!(report(&snapshot).is_err());
    }
}
