"""Azeroth Operator stdio MCP. No game control or arbitrary code execution."""
import os
from pathlib import Path
from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations
from shared.snapshots import SnapshotStore

configured_root = os.environ.get("AZEROTH_DATA_DIR")
if not configured_root or not Path(configured_root).is_absolute() or not Path(configured_root).is_dir():
    raise SystemExit("AZEROTH_DATA_DIR must name an existing absolute owner snapshot directory")
store = SnapshotStore(Path(configured_root))
mcp = FastMCP("Azeroth Operator")
READ = ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False)


@mcp.tool(annotations=READ)
def capabilities() -> dict:
    """Report implemented operations and policy restrictions; not Blizzard approval."""
    return {"families": ["retail", "classic", "forever"],
            "implemented": ["capabilities", "inspect_snapshot"],
            "game_execution": False, "live_client_qualified": [],
            "policy": "Read exported snapshots only. No unattended gameplay, input injection or protected-action bypass."}


@mcp.tool(annotations=READ)
def inspect_snapshot(snapshot_id: str, section: str | None = None) -> dict:
    """Inspect a stored snapshot by content hash. Always reports capture age, never claims live state."""
    return store.inspect(snapshot_id, section)


if __name__ == "__main__":
    mcp.run(transport="stdio")
