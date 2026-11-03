"""Read explicit installation metadata; never infer live API compatibility."""
import argparse
import json
from pathlib import Path
import re
import stat
from tools.sync_exports import ordinary, absolute


def read_metadata(path, limit):
    path = ordinary(path)
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError("Metadata must be a regular file")
    with path.open("rb") as handle:
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise ValueError("Metadata exceeds size limit")
    return data.decode("utf-8-sig")


def build_versions(text, product):
    lines = text.splitlines()
    if not lines:
        raise ValueError("Empty build metadata")
    headers = [field.split("!", 1)[0] for field in lines[0].split("|")]
    if len(headers) != len(set(headers)) or not {"Product", "Active", "Version"} <= set(headers):
        raise ValueError("Ambiguous or missing build columns")
    versions = set()
    for line in lines[1:]:
        if not line.strip():
            continue
        fields = line.split("|")
        if len(fields) != len(headers):
            raise ValueError("Malformed build row")
        row = dict(zip(headers, fields))
        if row["Product"] != product or row["Active"] != "1":
            continue
        if not re.fullmatch(r"[0-9]{1,4}(?:\.[0-9]{1,8}){3}", row["Version"]):
            raise ValueError("Malformed active build version")
        versions.add(row["Version"])
    return sorted(versions)


def inspect(client, family, variant):
    if family not in {"retail", "classic", "forever"}:
        raise ValueError("Unknown family")
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}", variant):
        raise ValueError("Explicit variant required")
    client = ordinary(absolute(client))
    result = {"family": family, "variant": variant, "variant_source": "caller",
              "live_qualified": False, "game_execution": False, "interface": None}
    if not client.exists():
        return result | {"status": "not_installed"}
    if not client.is_dir():
        raise ValueError("Expected client directory")
    flavor = read_metadata(client / ".flavor.info", 4096).splitlines()
    if len(flavor) != 2 or flavor[0] != "Product Flavor!STRING:0":
        raise ValueError("Malformed product metadata")
    product = flavor[1]
    allowed = {"retail": {"wow"}, "forever": {"wowf"},
               "classic": {"wow_classic", "wow_classic_era"}}
    if product not in allowed[family]:
        raise ValueError("Product does not match requested family")
    versions = build_versions(read_metadata(client.parent / ".build.info", 1048576), product)
    status = "identified" if len(versions) == 1 else "ambiguous_build" if versions else "no_active_build"
    return result | {"status": status, "product": product, "active_versions": versions,
                     "build": versions[0] if len(versions) == 1 else None}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", required=True)
    parser.add_argument("--family", required=True, choices=["retail", "classic", "forever"])
    parser.add_argument("--variant", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(inspect(args.client, args.family, args.variant)))
    except (ValueError, OSError, UnicodeError) as error:
        parser.exit(1, f"Client metadata rejected: {type(error).__name__}\n")
