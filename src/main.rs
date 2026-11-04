use azeroth_operator::{Family, Store};
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
struct Operator {
    store: Store,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Inspect {
    snapshot_id: String,
    section: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct List {
    family: String,
    offset: Option<usize>,
    limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Compare {
    before_id: String,
    after_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Summary {
    snapshot_id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentReview {
    snapshot_id: String,
    selections: Vec<azeroth_operator::planning::EquipmentSelection>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MacroOperation {
    snapshot_id: String,
    operation: azeroth_operator::macro_operation::MacroRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindingOperation {
    snapshot_id: String,
    operation: azeroth_operator::binding_operation::BindingRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentOperation {
    snapshot_id: String,
    operation: azeroth_operator::equipment_operation::EquipmentRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BagOperation {
    snapshot_id: String,
    operation: azeroth_operator::bag_operation::BagRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TalentOperation {
    snapshot_id: String,
    operation: azeroth_operator::talent_operation::TalentRequest,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ActionOperation {
    snapshot_id: String,
    operation: azeroth_operator::action_operation::ActionRequest,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentSetOperation {
    snapshot_id: String,
    operation: azeroth_operator::equipment_set_operation::EquipmentSetRequest,
}

#[tool_router]
impl Operator {
    #[tool(
        description = "Prepare Retail equipment-set operations. Create/save require addon 0.13.0-dev with player review/Apply, explicit ignored_slots and expect_empty_slots. Existing metadata, deletion and use requests are supported. Assign_spec/unassign_spec return association data with displaced associations requiring addon 0.12.0-dev and player review/Apply. Data only; never changes the game. Metadata requires addon 0.10.0-dev, use 0.11.0-dev and player review/Apply. Uncaptured equipped slots require explicit expect_empty_slots. Live execution remains unqualified. Does not repair external references.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_equipment_set_operation(
        &self,
        Parameters(args): Parameters<EquipmentSetOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::equipment_set_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment set operation rejected: captured set or requested metadata unavailable or invalid",
            )]),
        }
    }
    #[tool(
        description = "Prepare a Retail action-slot move/swap or copy into an empty slot from two explicitly observed slots, including exact macro contents. Data only; copy execution is disabled pending addon integration. Move/swap player review and Apply requires addon 0.9.0-dev or later. Live execution remains unqualified. Does not change action bars.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_action_operation(
        &self,
        Parameters(args): Parameters<ActionOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::action_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Action operation rejected: explicit slot or macro preimages unavailable or invalid",
            )]),
        }
    }

    #[tool(
        description = "Prepare Retail talent operations from captured contents. Load (default), rename and delete target a saved config; rename/delete require addon 0.14.0-dev. Create saves the current build under a new name, requires expected_new_shared_action_bars, forbids target_config_id and requires addon 0.15.0-dev. Creation selects the new build. Import accepts import_string plus explicit allow_empty_hash and expected_new_shared_action_bars, creates/selects a new build and changes active talents; requires addon 0.17.0-dev. Selected-loadout deletion requires addon 0.16.0-dev and clears saved selection while preserving current talent contents. No game changes; native qualification remains pending.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_talent_operation(
        &self,
        Parameters(args): Parameters<TalentOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::talent_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Talent operation rejected: committed or saved loadout observations unavailable",
            )]),
        }
    }
    #[tool(
        description = "Prepare a Retail carried-bag move or split to an explicitly empty slot. Omit amount for full-stack movement; provide a positive smaller amount to split. Binds captured GUID and count; player review and Apply required. Does not move items.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_bag_operation(&self, Parameters(args): Parameters<BagOperation>) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::bag_operation::compile(&snapshot, &args.snapshot_id, &args.operation)
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Bag operation rejected: source identity or empty destination precondition unavailable",
            )]),
        }
    }
    #[tool(
        description = "Prepare one exact-GUID Retail bag-to-equipment swap for player review. Requires item instance observations. Empty targets require explicit precondition. Does not equip anything; multi-slot handling remains unsupported.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_equipment_operation(
        &self,
        Parameters(args): Parameters<EquipmentOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::equipment_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment operation rejected: source/target identities or preconditions unavailable",
            )]),
        }
    }
    #[tool(
        description = "Compile a Retail keybinding change for explicit player review/application. Requires context and current binding set observations. For uncaptured keys, expect_unbound=true supplies a precondition checked live, not an observed fact. Empty desired action means unbind. Does not press keys or invoke actions.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_binding_operation(
        &self,
        Parameters(args): Parameters<BindingOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::binding_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Binding operation rejected: missing, ambiguous or invalid captured key/context/set",
            )]),
        }
    }
    #[tool(
        description = "Report implemented capabilities, not Blizzard approval",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn capabilities(&self) -> String {
        serde_json::json!({"families":["retail","classic","forever"],"implemented":["capabilities","inspect_snapshot","list_snapshots","compare_snapshots","character_summary","review_equipment_selection","captured_talent_loadout","prepare_macro_operation","prepare_binding_operation","prepare_equipment_operation", "prepare_bag_operation", "prepare_talent_operation", "prepare_action_operation", "prepare_equipment_set_operation"],"game_execution":false,"live_client_qualified":[]}).to_string()
    }
    #[tool(
        description = "Compile an explicit Retail macro create/edit/delete/scope move into data for player review and application in the addon. Existing macros are bound to captured name, icon and body. Does not execute in game or relocate action-bar references.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_macro_operation(
        &self,
        Parameters(args): Parameters<MacroOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::macro_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Macro operation rejected: snapshot, client, source or requested fields invalid",
            )]),
        }
    }
    #[tool(
        description = "Return the original captured Retail talent import string for player review, if available. Does not apply talents, create optimal builds or verify game acceptance.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn captured_talent_loadout(&self, Parameters(args): Parameters<Summary>) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            let mut review = azeroth_operator::planning::captured_loadout(&snapshot)?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
    #[tool(
        description = "Review player-selected equipment from exact exported locations. Returns a JSON draft; does not equip, optimise, validate game legality or generate a game import.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn review_equipment_selection(
        &self,
        Parameters(args): Parameters<EquipmentReview>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            let mut review =
                azeroth_operator::planning::equipment_review(&snapshot, &args.selections)?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment review rejected: snapshot/locations unavailable, mismatched, duplicated or invalid",
            )]),
        }
    }
    #[tool(
        description = "Compare same-character same-client snapshots; bounded changed JSON paths only",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn compare_snapshots(&self, Parameters(args): Parameters<Compare>) -> CallToolResult {
        let result = (|| {
            let a = self.store.read(&args.before_id)?;
            let b = self.store.read(&args.after_id)?;
            azeroth_operator::analysis::compare(&a, &b)
        })();
        match result {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshots unavailable, reversed or incompatible",
            )]),
        }
    }
    #[tool(
        description = "Summarise observed character data; missing values remain unknown",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn character_summary(&self, Parameters(args): Parameters<Summary>) -> CallToolResult {
        match self.store.read(&args.snapshot_id) {
            Ok(s) => CallToolResult::success(vec![ContentBlock::text(
                azeroth_operator::analysis::summary(&s).to_string(),
            )]),
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
    #[tool(
        description = "List snapshots for an explicit client family, newest capture first",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn list_snapshots(&self, Parameters(args): Parameters<List>) -> CallToolResult {
        let family = match args.family.as_str() {
            "retail" => Family::Retail,
            "classic" => Family::Classic,
            "forever" => Family::Forever,
            _ => return CallToolResult::error(vec![ContentBlock::text("Unknown family")]),
        };
        match self
            .store
            .list(family, args.offset.unwrap_or(0), args.limit.unwrap_or(20))
        {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshot listing failed or limit invalid",
            )]),
        }
    }
    #[tool(
        description = "Read a stored snapshot by hash, with capture age; never live state",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn inspect_snapshot(&self, Parameters(args): Parameters<Inspect>) -> CallToolResult {
        match self
            .store
            .inspect(&args.snapshot_id, args.section.as_deref())
        {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Operator {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("AZEROTH_DATA_DIR")
        .map(Into::into)
        .unwrap_or_else(|| std::path::PathBuf::from("local-data/snapshots"));
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.is_empty() {
        if args.len() == 4 && args[0] == "sync" {
            let family = match args[1].to_str() {
                Some("retail") => Family::Retail,
                Some("classic") => Family::Classic,
                Some("forever") => Family::Forever,
                _ => anyhow::bail!("Unknown sync family"),
            };
            println!(
                "{}",
                azeroth_operator::sync::sync_file(
                    std::path::Path::new(&args[2]),
                    family,
                    std::path::Path::new(&args[3])
                )
                .await?
            );
            return Ok(());
        }
        if args.len() == 2 && args[0] == "upload" {
            let snapshot = azeroth_operator::import::import_file(std::path::Path::new(&args[1]))?;
            println!("{}", azeroth_operator::upload::upload(&snapshot).await?);
            return Ok(());
        }
        let snapshot = if args.len() == 1 && args[0] == "import-stdin" {
            azeroth_operator::import::import_reader(std::io::stdin().lock())?
        } else if args.len() == 2 && args[0] == "import" {
            azeroth_operator::import::import_file(std::path::Path::new(&args[1]))?
        } else {
            anyhow::bail!(
                "Usage: azeroth-operator [import <file> | import-stdin | upload <file> | sync <family> <file> <receipt-dir>]"
            );
        };
        println!("{}", Store { root }.ingest(&snapshot)?);
        return Ok(());
    }
    let service = Operator {
        store: Store { root },
        tool_router: Operator::tool_router(),
    }
    .serve(rmcp::transport::stdio())
    .await?;
    service.waiting().await?;
    Ok(())
}
