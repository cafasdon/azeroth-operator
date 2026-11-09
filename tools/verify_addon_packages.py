"""Verify packaged bytes and execute each generated adapter through the Lua fixture."""
import argparse
import asyncio
import json
import os
from pathlib import Path
import subprocess
import tempfile
from tools.package_addon import render_entries
from tools.package_bigwigs import sha, verified_contents


async def readback_forever_fixture(binary, store, snapshot_id, closed_bank_id, receipt):
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client

    env = {**os.environ, "AZEROTH_DATA_DIR": str(store)}
    env.pop("AZEROTH_FOREVER_CATALOGUE", None)
    async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            async def call(tool, **arguments):
                reply = await session.call_tool(tool, arguments)
                if reply.isError:
                    raise ValueError(f"Fixture MCP call failed: {tool}")
                return json.loads(reply.content[0].text)

            sections = ["character_bank", "account_bank", "selected_recipe", "quests"]
            coverage = await call("inspect_snapshot", snapshot_id=snapshot_id, coverage_sections=sections)
            if (coverage["section_coverage"] != dict.fromkeys(sections, "observed")
                    or coverage["client"] != {"family": "forever", "variant": receipt["variant"],
                                             "interface": receipt["interface"], "build": "70862"}
                    or coverage["live"] is not False):
                raise ValueError("Unexpected fixture capture coverage or identity")
            stock = await call("find_snapshot_item", snapshot_id=snapshot_id, item_id=744)
            if (stock["quantity_by_storage"]["character_bank"] != 1
                    or stock["quantity_by_storage"]["account_bank"] != 0):
                raise ValueError("Unexpected fixture bank stock")
            recipe = await call("review_forever_recipe", snapshot_id=snapshot_id, crafts=2)
            materials = recipe["materials"]
            if (len(materials) != 1 or materials[0]["item_id"] != 744
                    or materials[0]["required"] != 6 or materials[0]["quantity_observed"] != 1
                    or materials[0]["shortfall_vs_observed"] != 5
                    or recipe["base_output_if_completed"] != {"item_id": 456, "minimum": 4, "maximum": 6}
                    or recipe["craftability_verified"] is not False):
                raise ValueError("Unexpected fixture recipe review")
            quest = await call("inspect_snapshot", snapshot_id=snapshot_id, section="quests", quest_id=42,
                               quest_inventory_requirements={"storage": "character_bank",
                                                             "items": [{"item_id": 744, "quantity": 3}]})
            inventory = quest["quest_inventory_review"]
            if (quest["quest_observed"] is not True
                    or inventory["items"] != [{"item_id": 744, "required_quantity": 3,
                                              "observed_quantity": 1, "shortfall_vs_observed": 2}]
                    or inventory["quest_item_binding_verified"] is not False):
                raise ValueError("Unexpected fixture quest inventory review")
            banks = ["character_bank", "account_bank"]
            closed = await call("inspect_snapshot", snapshot_id=closed_bank_id, coverage_sections=banks)
            if closed["section_coverage"] != dict.fromkeys(banks, "reported_unavailable"):
                raise ValueError("Closed fixture banks must be reported unavailable")
            closed_stock = await call("find_snapshot_item", snapshot_id=closed_bank_id, item_id=744)
            if any(closed_stock["quantity_by_storage"][bank] is not None for bank in banks):
                raise ValueError("Closed fixture bank quantities must be unknown")
            comparison = await call("compare_snapshots", before_id=snapshot_id, after_id=closed_bank_id)
            expected = {"status": "not_comparable", "before_available": True,
                        "after_available": False, "before_capture_status": "observed",
                        "after_capture_status": "reported_unavailable"}
            if any(comparison["sections"][bank] != expected for bank in banks):
                raise ValueError("Unexpected fixture bank comparison")


def verify(directory, verifier, mcp_binary=None):
    directory = Path(directory).resolve(strict=True)
    verifier = Path(verifier).resolve(strict=True)
    if mcp_binary is not None:
        mcp_binary = Path(mcp_binary).resolve(strict=True)
    manifests = sorted(directory.glob("*.manifest.json"))
    if not 1 <= len(manifests) <= 12:
        raise ValueError("Expected one to twelve package manifests")
    results = []
    for path in manifests:
        receipt = json.loads(path.read_text())
        name = receipt["archive"]
        if Path(name).name != name or not name.endswith(".zip"):
            raise ValueError("Invalid archive name")
        archive = directory / name
        if archive.is_symlink() or sha(archive.read_bytes()) != receipt["sha256"]:
            raise ValueError("Archive checksum mismatch")
        entries = render_entries(receipt["family"], receipt["variant"], receipt["interface"], receipt["version"])
        actual = verified_contents(archive, entries)
        if receipt["files"] != {name: sha(data) for name, data in actual.items()}:
            raise ValueError("File manifest mismatch")
        with tempfile.TemporaryDirectory(prefix="lua-fixture-", dir=directory) as temporary:
            stage = Path(temporary)
            for name, data in actual.items():
                (stage / name).write_bytes(data)
            command = [str(verifier), str(stage), receipt["family"], receipt["variant"], str(receipt["interface"])]
            readback = mcp_binary is not None and receipt["family"] == "forever"
            if readback:
                command.append("--fixture-export")
            run = subprocess.run(command,
                                 timeout=30, check=True, capture_output=True)
            result = json.loads(run.stdout)
            if readback:
                store = stage / "owner-store"
                store.mkdir()
                env = {**os.environ, "AZEROTH_DATA_DIR": str(store)}
                env.pop("AZEROTH_FOREVER_CATALOGUE", None)
                def import_fixture(key):
                    imported = subprocess.run([str(mcp_binary), "import-stdin"], env=env,
                        input=result.pop(key), text=True, timeout=15,
                        check=True, capture_output=True)
                    return imported.stdout.strip()

                snapshot_id = import_fixture("fixture_export")
                closed_bank_id = import_fixture("fixture_closed_bank_export")
                asyncio.run(asyncio.wait_for(
                    readback_forever_fixture(mcp_binary, store, snapshot_id, closed_bank_id, receipt), timeout=20))
                result["forever_mcp_fixture_readback"] = True
        if result.get("live_qualified") is not False or result.get("packaged_lua51_roundtrip") is not True:
            raise ValueError("Invalid fixture result")
        if receipt["family"] == "forever" and result.get("forever_capture_fixture") is not True:
            raise ValueError("Missing Forever capture fixture result")
        results.append({**result, "archive": receipt["archive"], "sha256": receipt["sha256"]})
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--verifier", type=Path, required=True)
    parser.add_argument("--mcp", type=Path, help="Verify Forever fixture import and readback through this local MCP binary")
    args = parser.parse_args()
    print(json.dumps(verify(args.directory, args.verifier, args.mcp), indent=2))
