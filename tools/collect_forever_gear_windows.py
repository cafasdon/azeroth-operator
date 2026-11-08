"""Collect bounded offline gear-plan windows and resume an explicitly bound journal."""
import argparse
import asyncio
import hashlib
from importlib.metadata import version
import json
import os
from pathlib import Path
import sys
import tempfile
from time import monotonic

from tools import journal_forever_gear_windows as journal
from tools import rank_forever_slot as ranker
from tools.merge_forever_gear_windows import digest, integer, parse_report, require


def file_digest(path, deadline):
    info = path.stat()
    require(path.is_file() and 0 < info.st_size <= 256 * 1024 * 1024)
    result = hashlib.sha256()
    consumed = 0
    with path.open('rb') as handle:
        while data := handle.read(1024 * 1024):
            require(monotonic() < deadline)
            consumed += len(data)
            require(consumed <= info.st_size)
            result.update(data)
    after = path.stat()
    require((info.st_size, info.st_mtime_ns) == (after.st_size, after.st_mtime_ns))
    return result.hexdigest()


def request_binding(args, deadline, template_sha256):
    root = Path(__file__).resolve().parent
    sources = {name: file_digest(root / name, deadline) for name in [
        'collect_forever_gear_windows.py', 'rank_forever_slot.py', 'run_forever_sim.py',
        'merge_forever_gear_windows.py', 'journal_forever_gear_windows.py']}
    config = {}
    for key in ['AZEROTH_DATA_DIR', 'AZEROTH_FOREVER_CATALOGUE']:
        raw = os.environ.get(key)
        path = Path(raw).resolve(strict=True) if raw else None
        config[key] = {'path': str(path) if path else None,
                       'sha256': file_digest(path, deadline) if path and path.is_file() else None}
    fields = ['snapshot_id', 'slot', 'collapse_identical_items', 'item_id', 'set_id', 'storage',
              'max_swaps', 'minimum_required_level', 'maximum_required_level',
              'observed_enchant_effect_id', 'observed_random_suffix_id', 'retain_equipment_specs']
    current_template = file_digest(args.template, deadline)
    require(current_template == template_sha256)
    request = {'version': 1, 'mcp_path': str(args.mcp), 'mcp_sha256': file_digest(args.mcp, deadline),
               'template_path': str(args.template), 'template_sha256': current_template,
               'python': sys.version, 'mcp_sdk': version('mcp'), 'sources': sources,
               'config': config, 'selection': {field: getattr(args, field) for field in fields}}
    return hashlib.sha256(json.dumps(request, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def validate(args):
    digest(args.snapshot_id)
    require(args.mcp.is_absolute() and args.mcp.is_file())
    args.mcp = args.mcp.resolve(strict=True)
    args.template = args.template.resolve(strict=True)
    args.journal = args.journal.parent.resolve(strict=True) / args.journal.name
    args.slot = list(args.slot)
    require(1 <= len(args.slot) <= 17 and len(set(args.slot)) == len(args.slot))
    integer(args.max_drafts, 1, 128)
    integer(args.window_size, 1, args.max_drafts)
    integer(args.max_windows, 1, 1024)
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.max_payload_mib, 1, 512)
    if args.item_id is not None:
        require(1 <= len(args.item_id) <= 16 and len(set(args.item_id)) == len(args.item_id))
        for item in args.item_id:
            integer(item, 1, 2**32 - 1)
    for value in [args.minimum_required_level, args.maximum_required_level]:
        if value is not None:
            integer(value, 0, 2**32 - 1)
    require(args.minimum_required_level is None or args.maximum_required_level is None
            or args.minimum_required_level <= args.maximum_required_level)
    for value in [args.set_id, args.observed_enchant_effect_id]:
        if value is not None:
            integer(value, 1, 2**32 - 1)
    if args.observed_random_suffix_id is not None:
        integer(args.observed_random_suffix_id, -(2**31), 2**31 - 1)


async def collect_windows(args):
    validate(args)
    deadline = monotonic() + args.time_budget_seconds
    with args.template.open('rb') as handle:
        template_data = handle.read(4 * 1024 * 1024 + 1)
        template = parse_report(template_data)
    template_sha256 = hashlib.sha256(template_data).hexdigest()
    require(isinstance(template, dict))
    ranker.check_consumable_groups(template)
    binding = request_binding(args, deadline, template_sha256)
    connection, _ = journal.connect(args.journal, write=True, deadline=deadline)
    connection.rollback()
    connection.close()
    status = journal.journal_status(args.journal, deadline=deadline) if args.journal.stat().st_size else None
    if status and status['window_count']:
        require(status.get('collection_binding_sha256') == binding and status['snapshot_id'] == args.snapshot_id)
    filters = {name: getattr(args, name) for name in ['minimum_required_level', 'maximum_required_level',
               'observed_enchant_effect_id', 'observed_random_suffix_id'] if getattr(args, name) is not None}
    inserted = attempted = 0
    stop = 'window_budget'
    with tempfile.TemporaryFile(mode='w+') as errlog:
        for _ in range(args.max_windows):
            if status and status['reported_coordinate_coverage_complete']:
                stop = 'reported_coverage_complete'
                break
            remaining = deadline - monotonic()
            if remaining <= 0:
                stop = 'time_budget'
                break
            require(request_binding(args, deadline, template_sha256) == binding)
            offset = int(status['next_uncovered_offset_decimal']) if status else 0
            limit = args.window_size
            if status and status['uncovered_ranges']:
                limit = min(limit, int(status['uncovered_ranges'][0]['end_decimal']) - offset)
            can_dual_wield = ranker.player_spec(ranker.single_player(template['raid'])) in ranker.DUAL_WIELD_SPECS
            remaining = deadline - monotonic()
            require(remaining > 0)
            collected = await asyncio.wait_for(ranker.collect(args.mcp, args.snapshot_id, args.slot, errlog,
                collapse_identical=args.collapse_identical_items, item_ids=args.item_id,
                can_dual_wield=can_dual_wield, set_id=args.set_id, max_drafts=args.max_drafts,
                storage=args.storage, max_swaps=args.max_swaps, candidate_filters=filters,
                combination_offset=offset, combination_limit=limit), timeout=min(45, remaining))
            drafts, skipped, coverage, stats, _, _, search = collected
            require(search['snapshot_id'] == args.snapshot_id and search['requested_slots'] == args.slot
                    and search['combination_window']['offset'] == offset
                    and search['combination_window']['limit'] == limit)
            identity = ranker.check_template_identity(template, stats, require_simulator_level=False)
            identity['hunter_pet'] = ranker.check_hunter_pet_level(ranker.single_player(template['raid']),
                drafts[0][1].get('captured_active_pet'), require_level=False)
            identity['professions'] = ranker.check_profession_identity(ranker.single_player(template['raid']),
                drafts[0][1].get('captured_professions'), require_match=False)
            require(request_binding(args, deadline, template_sha256) == binding)
            artifact = ranker.plan_report(drafts, include_gear_hashes=True,
                                          include_equipment_specs=args.retain_equipment_specs)
            artifact.update(gear_search=search, skipped=skipped, storage_coverage=coverage,
                            template_identity=identity, best_in_slot_verified=False,
                            collection_binding_sha256=binding)
            saved = journal.append_report(args.journal, artifact, args.max_payload_mib * 1024 * 1024, deadline=deadline)
            attempted += 1
            inserted += int(saved['inserted'])
            status = journal.journal_status(args.journal, deadline=deadline)
            require(status.get('collection_binding_sha256') == binding and status['snapshot_id'] == args.snapshot_id)
        if status and status['reported_coordinate_coverage_complete']:
            stop = 'reported_coverage_complete'
    return {'mode': 'gear_window_collection', 'collection_binding_sha256': binding,
            'windows_attempted': attempted, 'windows_inserted': inserted, 'stop_reason': stop,
            'progress': status, 'best_in_slot_verified': False, 'feasible_set_coverage_verified': False,
            'simulation_performed': False, 'game_execution': False}


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument('--mcp', required=True, type=Path, help='Absolute trusted offline MCP executable')
    result.add_argument('--snapshot-id', required=True)
    result.add_argument('--template', required=True, type=Path)
    result.add_argument('--journal', required=True, type=Path)
    slots = result.add_mutually_exclusive_group(required=True)
    slots.add_argument('--slot', action='append', choices=ranker.SIMULATOR_SLOTS)
    slots.add_argument('--all-slots', action='store_const', const=ranker.SIMULATOR_SLOTS, dest='slot')
    result.add_argument('--collapse-identical-items', action='store_true')
    result.add_argument('--retain-equipment-specs', action='store_true',
                        help='Persist numeric equipment inputs; does not qualify or run simulations')
    result.add_argument('--item-id', type=int, action='append')
    result.add_argument('--set-id', type=int)
    result.add_argument('--storage', choices=['equipment', 'bags', 'character_bank', 'account_bank'])
    result.add_argument('--max-swaps', type=int, choices=range(1, 18))
    for field in ['minimum-required-level', 'maximum-required-level', 'observed-enchant-effect-id', 'observed-random-suffix-id']:
        result.add_argument('--' + field, type=int)
    result.add_argument('--window-size', type=int, default=64)
    result.add_argument('--max-drafts', type=int, default=64)
    result.add_argument('--max-windows', type=int, default=4)
    result.add_argument('--time-budget-seconds', type=int, default=180)
    result.add_argument('--max-payload-mib', type=int, default=64)
    return result


def main(argv=None):
    cli = parser()
    args = cli.parse_args(argv)
    try:
        print(json.dumps(asyncio.run(collect_windows(args)), allow_nan=False))
    except KeyboardInterrupt:
        cli.exit(130, 'Gear collection interrupted; committed journal progress can be resumed.\n')
    except Exception:
        cli.exit(1, 'Gear collection stopped: invalid, changed or incompatible local inputs. Committed journal progress can be resumed.\n')


if __name__ == '__main__':
    main()
