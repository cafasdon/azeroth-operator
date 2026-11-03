"""Render a standalone Docker MCP profile; never read/export the shared profile."""
import argparse
import json
import re

TOOLS = ["capabilities", "inspect_snapshot", "list_snapshots",
         "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation","prepare_binding_operation","prepare_equipment_operation", "prepare_bag_operation", "prepare_talent_operation", "prepare_action_operation", "prepare_equipment_set_operation"]
STORE = "/var/lib/azeroth-operator/rodrigo/snapshots"


def profile(image):
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("An immutable image ID is required")
    server = {
        "name": "azeroth-operator", "type": "server", "image": image,
        "title": "Azeroth Operator",
        "description": "Read exported WoW snapshots; no gameplay execution",
        "env": [{"name": "AZEROTH_DATA_DIR", "value": "/data"}],
        "volumes": [f"{STORE}:/data:ro"],
    }
    return {
        "version": 1,
        "id": "azeroth-operator", "name": "azeroth-operator",
        "servers": [{"type": "image", "secrets": "default", "image": image,
                     "tools": list(TOOLS), "snapshot": {"server": server}}],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    args = parser.parse_args()
    print(json.dumps(profile(args.image), indent=2))
