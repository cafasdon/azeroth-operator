"""Render a standalone Docker MCP profile; never read/export the shared profile."""
import argparse
import json
from pathlib import PurePosixPath
import re

TOOLS = ["capabilities", "inspect_snapshot", "list_snapshots", "resolve_character",
         "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "review_forever_recipe", "search_forever_recipes", "find_snapshot_item", "plan_forever_recipe_restock"]


def snapshot_store(store):
    """Validate a literal Linux host path, not filesystem ownership or existence."""
    if (type(store) is not str or not 1 <= len(store) <= 4096
            or not store.startswith('/') or store.startswith('//')
            or ':' in store or '\\' in store
            or any(ord(char) < 32 or ord(char) == 127 for char in store)):
        raise ValueError('An explicit normalised Linux snapshot directory is required')
    path = PurePosixPath(store)
    if (path.as_posix() != store or len(path.parts) < 3
            or any(part in ('.', '..') or not re.fullmatch(r'[A-Za-z0-9._-]+', part)
                   for part in path.parts[1:])):
        raise ValueError('An explicit normalised Linux snapshot directory is required')
    return store


def profile(image, *, store):
    if type(image) is not str or not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("An immutable image ID is required")
    store = snapshot_store(store)
    server = {
        "name": "azeroth-operator", "type": "server", "image": image,
        "title": "Azeroth Operator",
        "description": "Read exported WoW snapshots; no gameplay execution",
        "env": [{"name": "AZEROTH_DATA_DIR", "value": "/data"}],
        "volumes": [f"{store}:/data:ro"],
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
    parser.add_argument("--store", required=True,
        help="Explicit dedicated snapshot directory on the Linux Docker host; no user default")
    args = parser.parse_args()
    try:
        rendered = profile(args.image, store=args.store)
    except ValueError as error:
        parser.error(str(error))
    print(json.dumps(rendered, indent=2))
