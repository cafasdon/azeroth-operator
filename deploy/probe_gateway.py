"""Probe loopback auth and MCP without disclosing the dedicated token."""
import json
import argparse
from pathlib import Path
import urllib.error
import urllib.request

URL = "http://127.0.0.1:8823/mcp"


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
        "capabilities", "inspect_snapshot", "list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation", "prepare_binding_operation", "prepare_equipment_operation", "prepare_bag_operation"}
    code, result, _ = request({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                              "params": {"name": "list_snapshots", "arguments": {"family": "retail"}}}, token, session)
    assert code == 200 and not result["result"].get("isError", False)
    snapshots = json.loads(result["result"]["content"][0]["text"])["snapshots"]
    assert isinstance(snapshots, list)
    print(json.dumps({"missing_token": 401, "wrong_token": 401, "authenticated": True,
                      "tools": 11, "retail_snapshots_returned": len(snapshots)}))
