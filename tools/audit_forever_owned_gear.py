"""Audit saved Forever inventory and simulator input resolution through the local MCP."""
import argparse
import asyncio
from pathlib import Path
from time import monotonic

from tools.evaluate_forever_owned_gear import add_snapshot_selector, catalogue_preflight, selected_snapshot
from tools.merge_forever_gear_windows import integer, require
from tools.prepare_forever_gear_window import encoded


async def audit(args):
    require(args.mcp.is_absolute())
    args.mcp = args.mcp.resolve(strict=True)
    require(args.mcp.is_file())
    if args.unresolved_item_limit is not None:
        integer(args.unresolved_item_limit, 1, 50)
    offset = 0
    if args.unresolved_item_offset is not None:
        require(args.unresolved_item_limit is not None)
        offset = integer(args.unresolved_item_offset, 0, 2**32-1)
    deadline = monotonic() + integer(args.time_budget_seconds, 1, 60)
    args.snapshot_id, selection = await selected_snapshot(args, deadline)
    evidence, counts, by_storage, unresolved = await catalogue_preflight(
        args, deadline, include_resolution_counts=True, unresolved_limit=args.unresolved_item_limit,
        unresolved_offset=offset)
    require(monotonic() <= deadline)
    report = {
        'mode': 'owned_gear_inventory_audit',
        'selection': selection,
        'snapshot_id': args.snapshot_id,
        'inventory_evidence': evidence,
        'preflight_simulator_item_resolution_counts': counts,
        'preflight_simulator_item_resolution_counts_by_storage': by_storage,
        'active_character_verified': False,
        'simulation_performed': False,
        'game_execution': False,
    }
    if unresolved is not None:
        unresolved['offset'] = offset
        unresolved['next_offset'] = offset + len(unresolved['locations']) if unresolved['has_more'] else None
        report['preflight_unresolved_item_locations'] = unresolved
    return report


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('--mcp', type=Path, required=True)
    add_snapshot_selector(cli)
    cli.add_argument('--unresolved-item-limit', type=int, help='Include up to 50 unresolved observed locations')
    cli.add_argument('--unresolved-item-offset', type=int, help='Start after this many unresolved locations; requires --unresolved-item-limit')
    cli.add_argument('--time-budget-seconds', type=int, default=30)
    args = cli.parse_args(argv)
    try:
        print(encoded(asyncio.run(audit(args))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Owned gear inventory audit interrupted.\n')
    except Exception:
        cli.exit(1, 'Owned gear inventory audit stopped: invalid, unsupported or over-budget local inputs.\n')


if __name__ == '__main__':
    main()
