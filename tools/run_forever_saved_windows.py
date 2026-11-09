"""Revalidate, deduplicate and execute bounded saved windows with private checkpoints."""
import argparse
import asyncio
from copy import deepcopy
import hashlib
from math import isfinite
from pathlib import Path
from time import monotonic

from tools import prepare_forever_gear_window as preparer
from tools import run_forever_sim as adapter
from tools.merge_forever_gear_windows import MAX_TOTAL_BYTES, digest, integer, require
from tools.merge_forever_prepared_windows import merge_prepared
from tools.simulation_journal import SimulationJournal


async def execute(args, *, deadline=None):
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.max_new_runs, 1, 128)
    integer(args.iteration_budget, 1, 100000)
    integer(args.simulation_journal_mib, 1, 512)
    require(1 <= len(args.window_offset) <= 64 and len(set(args.window_offset)) == len(args.window_offset))
    for offset in args.window_offset: integer(offset)
    if deadline is not None:
        require(type(deadline) in (int, float) and isfinite(deadline) and deadline > monotonic())
    deadline = min(monotonic() + args.time_budget_seconds, deadline) if deadline is not None else monotonic() + args.time_budget_seconds
    finalize_deadline = deadline + 5
    expected_hash = digest(args.engine_sha256.lower())
    require(args.engine.is_absolute())
    args.engine = args.engine.resolve(strict=True)
    require(preparer.file_digest(args.engine, deadline) == expected_hash)
    checkpoints = SimulationJournal(args.simulation_journal, args.simulation_journal_mib * 1024 * 1024)
    artifacts, preparation_bytes = [], 0
    for offset in args.window_offset:
        remaining = deadline - monotonic()
        require(remaining > 0)
        options = argparse.Namespace(mcp=args.mcp, journal=args.journal, window_offset=offset,
            template=args.template, encounter_lookup=args.encounter_lookup,
            time_budget_seconds=max(1, min(45, int(remaining))))
        artifact = await asyncio.wait_for(preparer.prepare(options), min(45, remaining))
        preparation_bytes += len(preparer.encoded(artifact))
        require(preparation_bytes <= MAX_TOTAL_BYTES)
        artifacts.append(artifact)
    work = merge_prepared(args.journal, artifacts)
    tank = args.rank_by in ('dtps', 'tps', 'death')
    healing = args.rank_by in ('hps', 'oom')
    bindings = [checkpoints.key(expected_hash, row['request'], args.paired, tank, healing) for row in work['requests']]
    work_key = hashlib.sha256(preparer.encoded({'keys': bindings, 'rank_by': args.rank_by,
        'snapshot': work['search_coverage']['snapshot_id'], 'scope': work['search_coverage']['scope_sha256']})).hexdigest()
    baseline = None
    rows, pending, leaders = [], [], []
    new_runs = new_iterations = hits = checked = row_bytes = 0
    stop = 'all_requested_results_available'
    for job, key in zip(work['requests'], bindings, strict=True):
        if monotonic() >= deadline:
            stop = 'time_budget'
            break
        adapter.verify_request_engine(args.engine,expected_hash,job['request'],deadline)
        summary = checkpoints.get(key)
        status = 'hit'
        if summary is None:
            iterations = job['request']['simOptions']['iterations']
            if getattr(args, 'checkpoint_only', False) or new_runs >= args.max_new_runs or new_iterations + iterations > args.iteration_budget:
                pending.append(job['global_review'])
                stop = ('checkpoint_only' if getattr(args, 'checkpoint_only', False) else
                        'run_budget' if new_runs >= args.max_new_runs else 'iteration_budget')
                checked += 1
                continue
            summary = adapter.run(args.engine, expected_hash, job['request'], deadline=deadline,
                paired=args.paired, tank_metrics=tank, healing_metrics=healing, cache=checkpoints)
            run_status = summary.pop('cache_status', None)
            require(run_status in ('miss', 'hit'))
            if run_status == 'miss':
                new_runs += 1
                new_iterations += iterations
                status = 'completed'
            else:
                hits += 1
        else:
            hits += 1
        checked += 1
        if args.paired and args.rank_by == 'dps':
            leaders.append((job, summary))
            leaders.sort(key=lambda item: (-item[1]['mean_dps'], item[0]['global_review']))
            del leaders[2:]
        if job['global_review'] == 1: baseline = summary
        if baseline is not None:
            requests = [work['requests'][0]['request'], job['request']]
            values = [baseline, summary]
            row = adapter.comparison_from_summaries(requests, values, args.paired, tank, healing, finalize_deadline)['results'][1]
        else:
            row = {field: value for field, value in summary.items() if not field.startswith('_')}
        row.update(review=job['global_review'], simulator_gear_sha256=job['simulator_gear_sha256'],
                   request_sha256=job['request_sha256'], references=job['references'], checkpoint_status=status)
        row_bytes += len(preparer.encoded(row))
        require(row_bytes <= MAX_TOTAL_BYTES)
        rows.append(row)
    uninspected = len(bindings) - checked
    complete = len(rows) == len(bindings)
    if complete: stop = 'all_requested_results_available'
    metric = {'death': 'simulated_deaths', 'oom': 'mean_seconds_oom'}.get(args.rank_by, f'mean_{args.rank_by}')
    rows.sort(key=lambda row: (-(row[metric]) if args.rank_by in ('dps', 'tps', 'hps') else row[metric], row['review']))
    require(preparer.file_digest(args.engine, finalize_deadline) == expected_hash)
    profile = deepcopy(work['requests'][0]['request'])
    adapter.single_player(profile['raid']).pop('equipment')
    profile.pop('encounter')
    profile.pop('simOptions')
    result = {'mode': 'saved_window_simulation', 'work_set_sha256': work_key, 'engine_sha256': expected_hash,
        'simulation_profile_sha256': hashlib.sha256(preparer.encoded(profile)).hexdigest(),
        'fingerprints': work['fingerprints'], 'encounter_settings': adapter.encounter_summary(work['requests'][0]['request']),
        'hunter_quiver_observation':work['hunter_quiver_observation'],
        'search_coverage': work['search_coverage'], 'requested_request_count': len(bindings),
        'completed_request_count': len(rows), 'uninspected_request_count': uninspected,
        'missing_request_count': len(pending), 'missing_reviews': pending[:128],
        'missing_reviews_truncated': len(pending) > 128, 'new_runs': new_runs, 'new_iterations': new_iterations,
        'checkpoint_hits': hits, 'stop_reason': stop, 'requested_results_complete': complete,
        'ranking_status': 'complete_for_selected_resolved_requests' if complete else 'incomplete',
        'baseline_delta_available': baseline is not None, 'ranked_by': metric, 'results': rows,
        'simulation_performed': new_runs > 0, 'game_execution': False,
        'best_in_slot_verified': False, 'feasible_set_coverage_verified': False}
    if len(leaders) == 2:
        pair = adapter.comparison_from_summaries([item[0]['request'] for item in leaders],
            [item[1] for item in leaders], paired=True, deadline=finalize_deadline)['observed_leading_dps_pair']
        for field in ('leading_review', 'runner_up_review'):
            pair[field] = leaders[pair[field] - 1][0]['global_review']
        result['observed_leading_dps_pair'] = pair
    require(len(preparer.encoded(result)) <= MAX_TOTAL_BYTES)
    return result


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'journal', 'template', 'encounter-lookup', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--window-offset', type=int, action='append', required=True)
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--checkpoint-only', action='store_true', help='Revalidate and inspect completed results without new engine runs')
    cli.add_argument('--rank-by', choices=['dps', 'dtps', 'tps', 'death', 'hps', 'oom'], default='dps')
    cli.add_argument('--max-new-runs', type=int, default=16)
    cli.add_argument('--iteration-budget', type=int, default=100000)
    cli.add_argument('--time-budget-seconds', type=int, default=180)
    cli.add_argument('--simulation-journal-mib', type=int, default=64)
    args = cli.parse_args(argv)
    try:
        print(preparer.encoded(asyncio.run(execute(args))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Saved simulation interrupted; completed numeric checkpoints remain available.\n')
    except Exception:
        cli.exit(1, 'Saved simulation stopped: invalid, changed, unsupported or over-budget local inputs; completed checkpoints remain available.\n')


if __name__ == '__main__':
    main()
