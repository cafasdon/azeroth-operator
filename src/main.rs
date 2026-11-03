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

#[tool_router]
impl Operator {
    #[tool(
        description = "Report implemented capabilities, not Blizzard approval",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn capabilities(&self) -> String {
        serde_json::json!({"families":["retail","classic","forever"],"implemented":["capabilities","inspect_snapshot","list_snapshots","compare_snapshots","character_summary"],"game_execution":false,"live_client_qualified":[]}).to_string()
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
                "Usage: azeroth-operator [import <export-file> | import-stdin | upload <export-file>]"
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
