"""Collect all-slot owned gear windows and simulate saved plans in one resumable offline run."""
import argparse
import asyncio
from copy import copy
import os
from pathlib import Path
import tempfile
from time import monotonic

from tools import collect_forever_gear_windows as collection
from tools import run_forever_saved_journal as simulation
from tools.merge_forever_gear_windows import MAX_REPORT_BYTES, MAX_TOTAL_BYTES, digest, integer, parse_report, require
from tools.prepare_forever_gear_window import encoded
from tools.sync_exports import ordinary
from tools import role_metrics as roles


async def selected_snapshot(args, deadline):
    snapshot = getattr(args, 'snapshot_id', None)
    name, realm = getattr(args, 'character_name', None), getattr(args, 'character_realm', None)
    if snapshot is not None:
        require(name is None and realm is None)
        return digest(snapshot), 'explicit_saved_snapshot'
    require(isinstance(name, str) and isinstance(realm, str) and name.strip() and realm.strip())
    require(monotonic() < deadline)
    ranker = collection.ranker
    async with asyncio.timeout(max(.001, min(15, deadline - monotonic()))):
        with tempfile.TemporaryFile(mode='w+') as errlog:
            params = ranker.StdioServerParameters(command=str(args.mcp), env=dict(os.environ))
            async with ranker.stdio_client(params, errlog=errlog) as (reader, writer):
                async with ranker.ClientSession(reader, writer) as session:
                    await session.initialize()
                    reply = await session.call_tool('resolve_character', {'family':'forever',
                        'character':{'name':name, 'realm':realm}})
                    require(not reply.isError and len(reply.content) == 1)
                    result = parse_report(reply.content[0].text.encode())
                    require(result['status'] == 'explicit_saved_character'
                        and result['live'] is False and result['active_character_verified'] is False)
                    return digest(result['selection']['snapshot_id']), 'explicit_saved_character'


async def execute(args):
    args = copy(args)
    args.candidate_scan_limit = collection.ranker.validate_candidate_scan_limit(getattr(args, 'candidate_scan_limit', 256))
    objective = simulation.ranking_objective(args)
    floor = simulation.encounter_improvement_floor(args)
    metric = roles.rank_by(args)
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.collection_seconds, 1, args.time_budget_seconds)
    for field, maximum in [('max_windows', 1024), ('max_pages', 1000000), ('page_limit', 64),
        ('top', 20), ('max_new_runs', 128), ('iteration_budget', 100000),
        ('max_payload_mib', 512), ('simulation_journal_mib', 512), ('window_size', 128)]:
        integer(getattr(args, field), 1, maximum)
    require(type(args.paired) is bool and type(args.checkpoint_only) is bool)
    deadline = monotonic() + args.time_budget_seconds
    expected_engine = digest(args.engine_sha256.lower())
    require(type(getattr(args,'positional_equipment',False)) is bool)
    require(type(getattr(args,'include_empty_slots',False)) is bool
        and (not getattr(args,'include_empty_slots',False) or getattr(args,'positional_equipment',False)))
    require(args.engine.is_absolute() and args.mcp.is_absolute())
    args.engine = args.engine.resolve(strict=True)
    args.mcp = args.mcp.resolve(strict=True)
    args.snapshot_id, selection = await selected_snapshot(args, deadline)
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
    require(monotonic() < deadline)
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
            'candidate_scan_limit':args.candidate_scan_limit,
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
    inventory_evidence = progress.get('inventory_evidence') if progress else None
    inventory_status = ('unavailable' if inventory_evidence is None else
        'observed_with_gaps' if inventory_evidence['unavailable_storage_sections']
            or inventory_evidence['catalogue_unmatched_item_locations'] else 'observed_without_reported_gaps')
    status = ('reported_space_complete' if complete else 'collection_incomplete' if not coordinates
        else 'unresolved_plans' if not resolved else 'simulation_incomplete')
    report = {'mode': 'owned_gear_evaluation', 'status': status,
        'selection': selection, 'snapshot_id': args.snapshot_id,
        'requested_slots': args.slot, 'candidate_storage_scope': 'all_observed_storage',
        'candidate_filters': {}, 'maximum_simultaneous_swaps': len(args.slot),
        'candidate_scan_limit': args.candidate_scan_limit,
        'equipment_layout':'azeroth_equipment_preserve_slots_v1' if getattr(args,'positional_equipment',False) else 'auto_place',
        'ranking_objective':objective,
        'minimum_encounter_improvement':floor,
        'rank_by':metric,'ranking_score_unit':roles.UNITS[metric],
        'inventory_evidence':inventory_evidence,'inventory_evidence_status':inventory_status,
        'unresolved_candidate_locations_by_slot': progress.get('unresolved_candidate_locations_by_slot') if progress else None,
        'collection': collected, 'simulation': ranked, 'simulation_stop_reason': reason,
        'reported_coordinate_coverage_complete': coordinates,
        'reported_proposals_resolved': resolved, 'all_saved_resolved_results_complete': simulated,
        'reported_space_results_complete': complete,
        'simulation_performed': bool(ranked and ranked['simulation_performed']),
        'active_character_verified': False, 'complete_ownership_verified': False,
        'global_ranking_verified': False, 'best_in_slot_verified': False,
        'feasible_set_coverage_verified': False, 'game_execution': False}
    if getattr(args,'include_empty_slots',False):
        report['empty_slot_choices_included'] = True
    require(len(encoded(report)) <= MAX_TOTAL_BYTES and monotonic() <= deadline + 5)
    return report


def parser():
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'template', 'journal', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    selector = cli.add_mutually_exclusive_group(required=True)
    selector.add_argument('--snapshot-id')
    selector.add_argument('--character-name')
    cli.add_argument('--character-realm', help='Required with --character-name; selects historical owner-store data')
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--encounter-lookup', type=Path, action='append', required=True)
    cli.add_argument('--encounter-weight', type=float, action='append')
    cli.add_argument('--ranking-objective',choices=simulation.RANKING_OBJECTIVES,default='weighted_gain',
        help='Rank by weighted average improvement or the minimum improvement across every supplied encounter')
    cli.add_argument('--rank-by',choices=roles.FIELDS,default='dps')
    cli.add_argument('--minimum-encounter-improvement',type=float,
        help='Require this improvement in every encounter before ranking; uses ranking_score_unit, including the baseline')
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--positional-equipment',action='store_true',help='Use the qualified hash/capability-checked exact-slot engine')
    cli.add_argument('--include-empty-slots',action='store_true',help='Include occupied-slot removal choices; requires positional equipment')
    cli.add_argument('--checkpoint-only', action='store_true', help='Reuse simulation checkpoints only; collection may add plan windows')
    cli.add_argument('--candidate-scan-limit', type=int, default=256,
        help='Maximum locations scanned per slot (1–4096, default 256); never truncates a pool')
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
