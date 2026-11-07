"""Probe loopback auth and MCP without disclosing the dedicated token."""
import json
import argparse
from pathlib import Path
import urllib.error
import urllib.request

URL = "http://127.0.0.1:8823/mcp"

CURRENT_INSPECTION_FIELDS = {
    "quest_id", "currency_id", "currency_reward_quantity", "faction_id",
    "achievement_id", "spending_budget",
}


def verify_current_inspection_schema(tools):
    inspection = next(tool for tool in tools if tool["name"] == "inspect_snapshot")
    schema = inspection["inputSchema"]
    properties = schema.get("properties")
    if schema.get("type") != "object" or not isinstance(properties, dict):
        raise ValueError("Gateway inspection schema must describe an object")
    expected = CURRENT_INSPECTION_FIELDS | {"snapshot_id", "section"}
    missing = expected - properties.keys()
    if missing:
        raise ValueError("Gateway inspection schema missing: " + ", ".join(sorted(missing)))
    if schema.get("required") != ["snapshot_id"]:
        raise ValueError("Gateway inspection schema changed required inputs")
    if not all(isinstance(properties[field], dict) and properties[field] for field in expected):
        raise ValueError("Gateway inspection schema contains invalid field definitions")


def request(payload, token=None, session=None):
    headers = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream",
               "MCP-Protocol-Version": "2025-11-25"}
    if token:
        headers["Authorization"] = "Bearer " + token
    if session:
        headers["Mcp-Session-Id"] = session
    req = urllib.request.Request(URL, json.dumps(payload).encode(), headers)
    try:
        with urllib.request.urlopen(req, timeout=20) as response:
            raw = response.read(1024 * 1024).decode()
            if raw.startswith("{"):
                body = json.loads(raw)
            else:
                messages = [json.loads(line[5:].strip()) for line in raw.splitlines() if line.startswith("data:")]
                body = messages[-1] if messages else None
            return response.status, body, response.headers.get("Mcp-Session-Id", session)
    except urllib.error.HTTPError as error:
        return error.code, None, session


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public", action="store_true")
    parser.add_argument("--require-current-inspection", action="store_true")
    args = parser.parse_args()
    if args.public:
        URL = "https://mcp.cafas.com.br/scopes/azeroth/mcp"
    token = Path("/etc/azeroth-operator/gateway.env").read_text().strip().split("=", 1)[1]
    initialise = {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "azeroth-probe", "version": "1"}}}
    assert request(initialise)[0] == 401, "Unauthenticated request accepted"
    assert request(initialise, "incorrect-token")[0] == 401, "Wrong token accepted"
    code, result, session = request(initialise, token)
    assert code == 200 and "result" in result, "Authenticated initialise failed"
    code, _, session = request({"jsonrpc": "2.0", "method": "notifications/initialized"}, token, session)
    assert code in (200, 202, 204)
    code, result, session = request({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}, token, session)
    assert code == 200 and {t["name"] for t in result["result"]["tools"]} == {
        "capabilities", "inspect_snapshot", "list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "review_forever_recipe", "find_snapshot_item"}
    tools = result["result"]["tools"]
    tool_count = len(tools)
    if args.require_current_inspection:
        verify_current_inspection_schema(tools)
    code, result, _ = request({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                              "params": {"name": "capabilities", "arguments": {}}}, token, session)
    assert code == 200 and not result["result"].get("isError", False)
    capabilities = json.loads(result["result"]["content"][0]["text"])
    assert capabilities["policy"]["mode"] == "offline_analysis"
    assert capabilities["policy"]["generated_game_imports"] is False
    assert capabilities["policy"]["snapshot_scope"] == "configured_owner_store"
    blocked = ("macro", "binding", "equipment", "bag", "talent", "action", "equipment_set")
    for request_id, operation in enumerate(blocked, start=5):
        code, result, _ = request({"jsonrpc": "2.0", "id": request_id, "method": "tools/call",
                                  "params": {"name": f"prepare_{operation}_operation", "arguments": {}}}, token, session)
        assert code == 200 and result is not None
        assert "error" in result or result.get("result", {}).get("isError") is True
        error_text = json.dumps(result.get("error", result.get("result"))).lower()
        assert any(reason in error_text for reason in ("tool not found", "unknown tool", "not allowed")), (
            f"Expected route denial for {operation}")
    code, result, _ = request({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                              "params": {"name": "list_snapshots", "arguments": {"family": "retail"}}}, token, session)
    assert code == 200 and not result["result"].get("isError", False)
    snapshots = json.loads(result["result"]["content"][0]["text"])["snapshots"]
    assert isinstance(snapshots, list)
    print(json.dumps({"missing_token": 401, "wrong_token": 401, "authenticated": True,
                      "tools": tool_count, "policy_mode": "offline_analysis", "prepare_routes_blocked": len(blocked), "retail_snapshots_returned": len(snapshots)}))
