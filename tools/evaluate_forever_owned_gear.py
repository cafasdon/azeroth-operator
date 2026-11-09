"""Collect all-slot owned gear windows and simulate saved plans in one resumable offline run."""
import argparse
import asyncio
from copy import copy
import os
from pathlib import Path
from time import monotonic

from tools import collect_forever_gear_windows as collection
from tools import run_forever_saved_journal as simulation
from tools.merge_forever_gear_windows import MAX_REPORT_BYTES, MAX_TOTAL_BYTES, digest, integer, parse_report, require
from tools.prepare_forever_gear_window import encoded
from tools.sync_exports import ordinary


async def execute(args):
    args = copy(args)
    objective = simulation.ranking_objective(args)
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.collection_seconds, 1, args.time_budget_seconds)
    for field, maximum in [('max_windows', 1024), ('max_pages', 1000000), ('page_limit', 64),
        ('top', 20), ('max_new_runs', 128), ('iteration_budget', 100000),
        ('max_payload_mib', 512), ('simulation_journal_mib', 512), ('window_size', 128)]:
        integer(getattr(args, field), 1, maximum)
    require(type(args.paired) is bool and type(args.checkpoint_only) is bool)
    digest(args.snapshot_id)
    expected_engine = digest(args.engine_sha256.lower())
    require(type(getattr(args,'positional_equipment',False)) is bool)
    require(args.engine.is_absolute() and args.mcp.is_absolute())
    args.engine = args.engine.resolve(strict=True)
    args.mcp = args.mcp.resolve(strict=True)
    args.template = args.template.resolve(strict=True)
    require(1 <= len(args.encounter_lookup) <= 8)
    args.encounter_lookup = [path.resolve(strict=True) for path in args.encounter_lookup]
    simulation.pages.encounters.normalise_weights(args.encounter_weight, len(args.encounter_lookup))
    args.journal = ordinary(args.journal.absolute())
    args.simulation_journal = ordinary(args.simulation_journal.absolute())
    args.journal = args.journal.parent.resolve(strict=True) / args.journal.name
    args.simulation_journal = args.simulation_journal.parent.resolve(strict=True) / args.simulation_journal.name
    require(not args.journal.exists() or args.journal.is_file())
    require(not args.simulation_journal.exists() or args.simulation_journal.is_dir())
    reserved = [ordinary(Path(str(args.journal) + suffix)) for suffix in ['', '-journal', '-wal', '-shm']]
    require(all(not path.resolve().is_relative_to(args.simulation_journal.resolve())
        and not args.simulation_journal.resolve().is_relative_to(path.resolve()) for path in reserved))
    inputs = [args.mcp, args.engine, args.template, *args.encounter_lookup]
    if os.environ.get('AZEROTH_FOREVER_CATALOGUE'):
        inputs.append(Path(os.environ['AZEROTH_FOREVER_CATALOGUE']).resolve(strict=True))
    if os.environ.get('AZEROTH_DATA_DIR'):
        inputs.append((Path(os.environ['AZEROTH_DATA_DIR']) / (args.snapshot_id + '.json')).resolve(strict=True))
    for path in inputs:
        require(path.is_file() and all(path != output.resolve()
            and (not output.exists() or not path.samefile(output)) for output in reserved)
            and not path.is_relative_to(args.simulation_journal.resolve())
        )
    deadline = monotonic() + args.time_budget_seconds
    require(collection.file_digest(args.engine, deadline) == expected_engine)
    if getattr(args,'positional_equipment',False):
        simulation.pages.encounters.saved.adapter.verify_positional_engine(args.engine,expected_engine,deadline)
    for path in [args.template, *args.encounter_lookup]:
        with path.open('rb') as handle:
            require(isinstance(parse_report(handle.read(MAX_REPORT_BYTES + 1)), dict))
    args.slot = list(collection.ranker.SIMULATOR_SLOTS)
    args.collapse_identical_items = args.retain_equipment_specs = True
    args.max_drafts = args.window_size
    for field in ['item_id', 'set_id', 'storage', 'max_swaps', 'minimum_required_level',
        'maximum_required_level', 'observed_enchant_effect_id', 'observed_random_suffix_id']:
        setattr(args, field, None)
    collection_deadline = min(deadline, monotonic() + args.collection_seconds)
    try:
        collected = await collection.collect_windows(copy(args), deadline=collection_deadline)
    except Exception:
        # Stdio cancellation can surface as an ExceptionGroup, not TimeoutError.
        # Only expiry of this stage permits recovery; early failures still reject.
        require(monotonic() >= collection_deadline and monotonic() < deadline)
        binding = collection.request_binding(args, deadline, collection.file_digest(args.template, deadline))
        progress = collection.journal.journal_status(args.journal, deadline=deadline) if args.journal.stat().st_size else None
        if progress and progress['window_count']:
            require(progress['collection_binding_sha256'] == binding and progress['snapshot_id'] == args.snapshot_id)
        collected = {'mode':'gear_window_collection','collection_binding_sha256':binding,
            'windows_attempted':None,'windows_inserted':None,'stop_reason':'time_budget',
            'progress':progress,'interrupted_stage_recovered':True,
            'best_in_slot_verified':False,'feasible_set_coverage_verified':False,
            'simulation_performed':False,'game_execution':False}
    progress = collected['progress']
    ranked = None
    reason = 'no_saved_windows'
    if progress and progress['window_count']:
        reason = 'time_budget'
        if monotonic() < deadline:
            ranked = await simulation.execute(copy(args), deadline=deadline)
            require(ranked['journal_coverage'] == progress)
            reason = ranked['stop_reason']
    coordinates = bool(progress and progress['reported_coordinate_coverage_complete'])
    resolved = bool(coordinates and progress['resolution_accounting']['unresolved_proposed_coordinates'] == 0)
    simulated = bool(ranked and ranked['all_saved_resolved_results_complete'])
    complete = coordinates and resolved and simulated
    status = ('reported_space_complete' if complete else 'collection_incomplete' if not coordinates
        else 'unresolved_plans' if not resolved else 'simulation_incomplete')
    report = {'mode': 'owned_gear_evaluation', 'status': status,
        'selection': 'explicit_saved_snapshot', 'snapshot_id': args.snapshot_id,
        'requested_slots': args.slot, 'candidate_storage_scope': 'all_observed_storage',
        'candidate_filters': {}, 'maximum_simultaneous_swaps': len(args.slot),
        'equipment_layout':'azeroth_equipment_preserve_slots_v1' if getattr(args,'positional_equipment',False) else 'auto_place',
        'ranking_objective':objective,
        'collection': collected, 'simulation': ranked, 'simulation_stop_reason': reason,
        'reported_coordinate_coverage_complete': coordinates,
        'reported_proposals_resolved': resolved, 'all_saved_resolved_results_complete': simulated,
        'reported_space_results_complete': complete,
        'simulation_performed': bool(ranked and ranked['simulation_performed']),
        'active_character_verified': False, 'complete_ownership_verified': False,
        'global_ranking_verified': False, 'best_in_slot_verified': False,
        'feasible_set_coverage_verified': False, 'game_execution': False}
    require(len(encoded(report)) <= MAX_TOTAL_BYTES and monotonic() <= deadline + 5)
    return report


def parser():
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'template', 'journal', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    cli.add_argument('--snapshot-id', required=True)
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--encounter-lookup', type=Path, action='append', required=True)
    cli.add_argument('--encounter-weight', type=float, action='append')
    cli.add_argument('--ranking-objective',choices=simulation.RANKING_OBJECTIVES,default='weighted_gain',
        help='Rank by weighted average gain or the minimum gain across every supplied encounter')
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--positional-equipment',action='store_true',help='Use the qualified hash/capability-checked exact-slot engine')
    cli.add_argument('--checkpoint-only', action='store_true', help='Reuse simulation checkpoints only; collection may add plan windows')
    for name, default in [('window-size', 64), ('max-windows', 4), ('max-pages', 64),
        ('page-limit', 8), ('top', 10), ('max-new-runs', 16), ('iteration-budget', 100000),
        ('time-budget-seconds', 180), ('collection-seconds', 45), ('max-payload-mib', 64),
        ('simulation-journal-mib', 64)]:
        cli.add_argument('--' + name, type=int, default=default)
    return cli


def main(argv=None):
    cli = parser()
    try:
        print(encoded(asyncio.run(execute(cli.parse_args(argv)))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Owned gear evaluation interrupted; committed plans and numeric checkpoints remain available.\n')
    except Exception:
        cli.exit(1, 'Owned gear evaluation stopped: invalid, changed, unsupported or over-budget local inputs; committed work remains available.\n')


if __name__ == '__main__':
    main()
