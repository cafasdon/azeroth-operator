"""Run selected saved gear windows across encounters under one shared work budget."""
import argparse
import asyncio
from copy import copy
from math import fsum, isfinite
from pathlib import Path
from time import monotonic

from tools import run_forever_saved_windows as saved
from tools.merge_forever_gear_windows import MAX_TOTAL_BYTES, digest, integer, require
from tools.merge_forever_prepared_windows import FINGERPRINTS
from tools.prepare_forever_gear_window import encoded
from tools.rank_forever_slot import weighted_ranking, encounter_sensitivity
from tools import role_metrics as roles


def normalise_weights(weights, count):
    weights = [1.0] * count if weights is None else weights
    require(len(weights) == count and all(type(value) in (int, float) and isfinite(value) and value > 0 for value in weights))
    largest = max(weights)
    scaled = [value / largest for value in weights]
    total = fsum(scaled)
    result = [value / total for value in scaled]
    require(all(value > 0 for value in result))
    return result


def completed_candidate_ranking(results, weights, metric, requested_count, gear,
                                objective='weighted_gain', top=20):
    """Rank only the intersection observed against a baseline in every encounter."""
    require(objective in ('weighted_gain', 'worst_encounter_gain'))
    integer(top, 1, 20)
    integer(requested_count, 1)
    if len(results) != len(weights) or not results:
        return None
    profiles = []
    for result in results:
        rows = {}
        for row in result['results']:
            review = integer(row['review'], 1, requested_count)
            require(review not in rows)
            rows[review] = row
        if 1 not in rows or metric == 'dps' and roles.value(rows[1], metric) <= 0:
            return None
        profiles.append(rows)
    common = set.intersection(*(set(rows) for rows in profiles))
    subset = [{'baseline_review':1, 'results':[rows[review] for review in sorted(common)]}
              for rows in profiles]
    ranking = (weighted_ranking(subset, weights) if metric == 'dps'
               else roles.weighted_ranking(subset, weights, metric))
    score_field = 'weighted_dps_gain_pct' if metric == 'dps' else 'weighted_improvement'
    gain_field = 'gain_pct_by_encounter' if metric == 'dps' else 'improvement_by_encounter'
    value_field = 'mean_dps_by_encounter' if metric == 'dps' else 'metric_values_by_encounter'
    rows = []
    for ranked in ranking['results']:
        review = ranked['review']
        gains = ([100 * profile[review]['mean_dps_delta_from_baseline'] / profile[1]['mean_dps']
                  for profile in profiles] if metric == 'dps' else ranked[gain_field])
        values = [roles.value(profile[review], metric) for profile in profiles]
        score = ranked[score_field]
        require(all(type(value) in (int,float) and isfinite(value) for value in [score,*gains,*values]))
        rows.append({'review':review, 'simulator_gear_sha256':digest(gear[review]),
            score_field:score, gain_field:gains, value_field:values,
            'ranking_score':score if objective == 'weighted_gain' else min(gains)})
    rows.sort(key=lambda row:(-row['ranking_score'], -row[score_field], row['simulator_gear_sha256']))
    return {'basis':'completed_candidates_across_all_requested_encounters',
        'rank_by':metric, 'ranking_objective':objective, 'score_unit':roles.UNITS[metric],
        'requested_candidate_count':requested_count, 'completed_candidate_count':len(common),
        'omitted_candidate_count':requested_count-len(common), 'top_limit':top,
        'results_truncated':len(rows)>top, 'results':rows[:top],
        'requested_results_complete':False, 'global_ranking_verified':False,
        'best_in_slot_verified':False, 'feasible_set_coverage_verified':False,
        'aggregate_ranking_uncertainty_verified':False, 'game_execution':False}


async def execute(args, *, deadline=None):
    metric = roles.rank_by(args)
    objective = getattr(args, 'ranking_objective', 'weighted_gain')
    require(objective in ('weighted_gain', 'worst_encounter_gain'))
    top = integer(getattr(args, 'top', 20), 1, 20)
    count = len(args.encounter_lookup)
    require(1 <= count <= 8)
    weights = normalise_weights(args.encounter_weight, count)
    integer(args.max_new_runs, 1, 128)
    integer(args.iteration_budget, 1, 100000)
    integer(args.time_budget_seconds, 1, 3600)
    if deadline is not None:
        require(type(deadline) in (int, float) and isfinite(deadline) and deadline > monotonic())
    deadline = min(monotonic() + args.time_budget_seconds, deadline) if deadline is not None else monotonic() + args.time_budget_seconds
    profiles, results, identities, gear = [], [], None, {}
    new_runs = new_iterations = hits = size = 0
    for index, path in enumerate(args.encounter_lookup, 1):
        remaining = deadline - monotonic()
        if remaining <= 0: break
        options = copy(args)
        options.encounter_lookup = path
        options.rank_by = metric
        options.time_budget_seconds = args.time_budget_seconds
        options.max_new_runs = max(1, args.max_new_runs - new_runs)
        options.iteration_budget = max(1, args.iteration_budget - new_iterations)
        options.checkpoint_only = (getattr(args, 'checkpoint_only', False)
            or new_runs >= args.max_new_runs or new_iterations >= args.iteration_budget)
        result = await saved.execute(options, deadline=deadline)
        if metric != 'dps': require(result['ranked_by'] == ('simulated_deaths' if metric=='death' else roles.FIELDS[metric]))
        require(set(result['fingerprints']) == set(FINGERPRINTS))
        current = {'engine': digest(result['engine_sha256']), 'profile': digest(result['simulation_profile_sha256']),
            'fingerprints': {key: digest(value) for key, value in result['fingerprints'].items() if key != 'encounter_sha256'},
            'hunter_quiver_observation':result.get('hunter_quiver_observation'),
            'coverage': result['search_coverage'], 'count': result['requested_request_count']}
        if identities is None: identities = current
        require(encoded(current) == encoded(identities))
        new_runs += result['new_runs']
        new_iterations += result['new_iterations']
        hits += result['checkpoint_hits']
        require(new_runs <= args.max_new_runs and new_iterations <= args.iteration_budget)
        local = {}
        for row in result['results']:
            review = integer(row['review'], 1, result['requested_request_count'])
            require(review not in local)
            local[review] = digest(row['simulator_gear_sha256'])
            require(review not in gear or gear[review] == local[review])
            gear[review] = local[review]
        result['baseline_review'] = 1
        profile = {'encounter_index': index, 'weight': weights[index - 1], 'result': result}
        size += len(encoded(profile))
        require(size <= MAX_TOTAL_BYTES)
        profiles.append(profile)
        results.append(result)
    require(monotonic() <= deadline + 5)
    complete = len(profiles) == count and all(result['requested_results_complete'] is True for result in results)
    weighted = sensitivity = metric_ranking = None
    status = 'incomplete'
    if complete:
        expected = set(range(1, identities['count'] + 1))
        require(all({row['review'] for row in result['results']} == expected for result in results))
        if metric == 'dps': sensitivity = encounter_sensitivity(results, identities['count'])
        if metric != 'dps':
            metric_ranking = roles.weighted_ranking(results, weights, metric)
            for row in metric_ranking['results']: row['simulator_gear_sha256'] = gear[row['review']]
            status = 'complete_for_selected_resolved_requests'
        elif all(next(row['mean_dps'] for row in result['results'] if row['review'] == 1) > 0 for result in results):
            weighted = weighted_ranking(results, weights)
            require(all(isfinite(row['weighted_dps_gain_pct']) for row in weighted['results']))
            for row in weighted['results']: row['simulator_gear_sha256'] = gear[row['review']]
            status = 'complete_for_selected_resolved_requests'
        else:
            status = 'nonpositive_baseline'
    partial = (completed_candidate_ranking(results, weights, metric, identities['count'], gear, objective, top)
               if not complete and identities is not None else None)
    report = {'mode': 'saved_window_encounter_comparison', 'encounter_weights': weights,
        'completed_candidate_ranking':partial,
        'rank_by':metric,'metric_ranking':metric_ranking,
        'requested_encounter_count': count, 'visited_encounter_count': len(profiles),
        'unvisited_encounters': list(range(len(profiles) + 1, count + 1)),
        'completed_encounter_count': sum(result['requested_results_complete'] is True for result in results),
        'requested_results_complete': complete, 'weighted_status': status, 'weighted_ranking': weighted,
        'encounter_sensitivity': sensitivity, 'profiles': profiles, 'new_runs': new_runs,
        'new_iterations': new_iterations, 'checkpoint_hits': hits, 'simulation_performed': new_runs > 0,
        'game_execution': False, 'best_in_slot_verified': False, 'feasible_set_coverage_verified': False}
    report_bytes = len(encoded(report))
    require(report_bytes <= MAX_TOTAL_BYTES and monotonic() <= deadline + 5)
    return report


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'journal', 'template', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    cli.add_argument('--encounter-lookup', type=Path, action='append', required=True)
    cli.add_argument('--encounter-weight', type=float, action='append')
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--window-offset', type=int, action='append', required=True)
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--rank-by',choices=roles.FIELDS,default='dps')
    cli.add_argument('--ranking-objective', choices=['weighted_gain', 'worst_encounter_gain'], default='weighted_gain',
        help='Objective for the explicitly incomplete completed-candidate ranking')
    cli.add_argument('--checkpoint-only', action='store_true')
    cli.add_argument('--max-new-runs', type=int, default=16)
    cli.add_argument('--iteration-budget', type=int, default=100000)
    cli.add_argument('--time-budget-seconds', type=int, default=180)
    cli.add_argument('--simulation-journal-mib', type=int, default=64)
    args = cli.parse_args(argv)
    try:
        print(encoded(asyncio.run(execute(args))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Encounter comparison interrupted; numeric checkpoints remain available.\n')
    except Exception:
        cli.exit(1, 'Encounter comparison stopped: invalid, changed, unsupported or over-budget local inputs; numeric checkpoints remain available.\n')


if __name__ == '__main__':
    main()
