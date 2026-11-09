"""Execute saved journal pages and rank completed gear across a stable journal."""
import argparse
import asyncio
from copy import copy
from math import isfinite
from pathlib import Path
from time import monotonic

from tools import journal_forever_gear_windows as journal
from tools import run_forever_journal_page as pages
from tools.merge_forever_gear_windows import MAX_TOTAL_BYTES, digest, integer, require
from tools.prepare_forever_gear_window import encoded
from tools.prepare_forever_gear_window import selections_for_review, cleared_for_review
from tools.merge_forever_gear_windows import retained_equipment
from tools import role_metrics as roles

RANKING_OBJECTIVES = ('weighted_gain', 'worst_encounter_gain')


def encounter_evidence(row, baseline, metric, index):
    """Retain public numeric evidence; never export private iteration samples."""
    fields = ['iterations', 'seed', 'request_sha256', roles.FIELDS[metric]]
    if metric in ('dps', 'dtps', 'tps', 'hps'):
        fields.extend([f'{metric}_standard_deviation',
            f'paired_{metric}_standard_error',
            f'paired_{metric}_delta_interval_approx_95'])
    if metric == 'death':
        fields.extend(['simulated_deaths', 'simulated_death_fraction_wilson_95'])
    return {'encounter_index': index, 'comparison_basis': 'equipped_baseline',
        'metric': metric, 'baseline_metric_value': roles.value(baseline, metric),
        'baseline_request_sha256': baseline['request_sha256'],
        'interval_basis': ('marginal_death_fraction' if metric == 'death'
            and 'simulated_death_fraction_wilson_95' in row else
            'candidate_minus_baseline' if f'paired_{metric}_delta_interval_approx_95' in row
            else 'not_available'),
        'aggregate_ranking_uncertainty_verified': False,
        'selection_adjusted': False,
        **{field: row[field] for field in fields if field in row}}


def ranking_objective(args):
    objective = getattr(args, 'ranking_objective', 'weighted_gain')
    require(isinstance(objective,str) and objective in RANKING_OBJECTIVES)
    return objective


def attach_saved_loadouts(path, leaders, source, deadline):
    """Join bounded ranking references to checked historical plans, never equip."""
    require(isinstance(leaders, dict) and len(leaders) <= 20)
    if not leaders:
        return
    connection, deadline = journal.connect(path, deadline=deadline)
    try:
        for gear, leader in leaders.items():
            require(monotonic() < deadline)
            require(digest(gear) == leader['simulator_gear_sha256'])
            offset = leader['source_window_offset_decimal']
            require(isinstance(offset,str) and 1 <= len(offset) <= 64 and offset.isascii()
                and offset.isdecimal() and str(int(offset)) == offset)
            row = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows WHERE start_key=?',
                (journal.coordinate(int(offset)),)).fetchone()
            require(row is not None)
            report, summary = journal.checked_row(row)
            require(all(summary.get(field) == source.get(field) for field in
                ['snapshot_id','scope_sha256','collection_binding_sha256']))
            review = integer(leader['source_window_review'],1,len(report['resolved_drafts']))
            draft = report['resolved_drafts'][review-1]
            require(draft['review'] == review and draft['simulator_gear_sha256'] == gear)
            spec = retained_equipment(draft)
            require(spec is not None)
            selections = selections_for_review(draft['selections'])
            cleared = cleared_for_review(draft)
            leader['saved_loadout'] = {'basis':'validated_saved_plan',
                'snapshot_id':summary['snapshot_id'],'source_report_sha256':row[3],
                'simulator_equipment_spec':spec,'selections':selections,
                'cleared_equipment_slots':cleared,
                'current_locations_verified':False,'native_equip_eligibility_verified':False,
                'game_execution':False}
            if draft.get('explicit_clear_equipment_slots'):
                leader['saved_loadout']['explicit_clear_equipment_slots'] = draft['explicit_clear_equipment_slots']
    finally:
        connection.close()


async def execute(args, *, deadline=None):
    objective = ranking_objective(args)
    metric = roles.rank_by(args)
    weighted_field = 'weighted_dps_gain_pct' if metric=='dps' else 'weighted_improvement'
    gain_field = 'gain_pct_by_encounter' if metric=='dps' else 'improvement_by_encounter'
    value_field = 'mean_dps_by_encounter' if metric=='dps' else 'metric_values_by_encounter'
    minimum_field = 'minimum_dps_gain_pct' if metric=='dps' else 'minimum_improvement'
    maximum_field = 'maximum_dps_gain_pct' if metric=='dps' else 'maximum_improvement'
    integer(args.max_pages, 1, 1000000)
    integer(args.top, 1, 20)
    integer(args.page_limit, 1, 64)
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.max_new_runs, 1, 128)
    integer(args.iteration_budget, 1, 100000)
    integer(args.simulation_journal_mib, 1, 512)
    require(1 <= len(args.encounter_lookup) <= 8)
    weights = pages.encounters.normalise_weights(args.encounter_weight, len(args.encounter_lookup))
    digest(args.engine_sha256.lower())
    if deadline is not None:
        require(type(deadline) in (int, float) and isfinite(deadline) and deadline > monotonic())
    deadline = min(monotonic() + args.time_budget_seconds, deadline) if deadline is not None else monotonic() + args.time_budget_seconds
    source = journal.journal_status(args.journal, deadline=deadline)
    require(not source['window_count'] or source['equipment_specs_retained'] is True)
    cursor = None
    leaders, contexts, baselines = {}, {}, {}
    baseline_gear = None
    visited = completed = new_runs = new_iterations = hits = 0
    page_count = 0
    progress = []
    stop = 'no_saved_work' if source['window_count'] == 0 else 'page_budget'
    exhausted = False
    incomplete_page_candidates = None
    for _ in range(args.max_pages if source['window_count'] else 0):
        if monotonic() >= deadline:
            stop = 'time_budget'
            break
        options = copy(args)
        options.after_window_offset = cursor
        options.max_new_runs = max(1, args.max_new_runs - new_runs)
        options.iteration_budget = max(1, args.iteration_budget - new_iterations)
        options.checkpoint_only = (getattr(args, 'checkpoint_only', False)
            or new_runs >= args.max_new_runs or new_iterations >= args.iteration_budget)
        page = await pages.execute(options, deadline=deadline)
        selected = page['journal_page']
        count = selected['selected_window_count']
        require(count > 0)
        visited += count
        page_count += 1
        comparison = page['encounter_comparison']
        require(comparison['encounter_weights'] == weights)
        new_runs += comparison['new_runs']
        new_iterations += comparison['new_iterations']
        hits += comparison['checkpoint_hits']
        require(new_runs <= args.max_new_runs and new_iterations <= args.iteration_budget)
        profiles = comparison['profiles']
        for profile in profiles:
            index, result = profile['encounter_index'], profile['result']
            coverage = result['search_coverage']
            require(coverage['snapshot_id'] == source['snapshot_id'] and coverage['scope_sha256'] == source['scope_sha256'])
            signature = {'engine': result['engine_sha256'], 'profile': result['simulation_profile_sha256'],
                'fingerprints': result['fingerprints'], 'settings': result['encounter_settings']}
            require(index not in contexts or contexts[index] == signature)
            contexts[index] = signature
            baseline = next((row for row in result['results'] if row['review'] == 1), None)
            if baseline is not None:
                identity = {key: baseline[key] for key in ['simulator_gear_sha256', 'request_sha256',
                    'iterations', 'seed', 'mean_dps', 'dps_standard_deviation']}
                if metric != 'dps': identity[roles.FIELDS[metric]] = roles.value(baseline,metric)
                require(index not in baselines or baselines[index] == identity)
                baselines[index] = identity
                require(baseline_gear is None or baseline_gear == baseline['simulator_gear_sha256'])
                baseline_gear = baseline['simulator_gear_sha256']
        if len(progress) < 64:
            progress.append({'after_window_offset_decimal': str(cursor) if cursor is not None else None,
                'selected_window_count': count, 'page_status': page['page_status'],
                'weighted_status': comparison['weighted_status']})
        page_complete = selected['page_results_complete']
        if not page_complete:
            stop = 'incomplete_page'
            ranking = comparison.get('completed_candidate_ranking')
            if ranking is None:
                break
            require(ranking['ranking_objective'] == objective and ranking['rank_by'] == metric)
            incomplete_page_candidates = ranking['completed_candidate_count']
        else:
            completed += count
            if comparison['weighted_status'] != 'complete_for_selected_resolved_requests':
                stop = comparison['weighted_status']
                break
            ranking = comparison['weighted_ranking'] if metric=='dps' else comparison['metric_ranking']
        first = {row['review']: row for row in profiles[0]['result']['results']}
        metrics = [{row['review']: row for row in profile['result']['results']} for profile in profiles]
        for weighted in ranking['results']:
            review = weighted['review']
            gear = digest(weighted['simulator_gear_sha256'])
            score = weighted[weighted_field]
            require(type(score) in (int, float) and isfinite(score))
            reference = first[review]['references'][0]
            row = {'simulator_gear_sha256': gear, weighted_field: score,
                value_field: ([rows[review]['mean_dps'] for rows in metrics] if metric=='dps'
                    else weighted[value_field]),
                gain_field: ([100 * rows[review]['mean_dps_delta_from_baseline'] / rows[1]['mean_dps'] for rows in metrics]
                    if metric=='dps' else weighted[gain_field]),
                'source_window_offset_decimal': selected['selected_window_offsets_decimal'][reference['window_index'] - 1],
                'source_window_review': reference['review']}
            row['simulation_evidence_by_encounter'] = [
                encounter_evidence(rows[review], rows[1], metric, index)
                for index, rows in enumerate(metrics, 1)]
            require(all(type(gain) in (int,float) and isfinite(gain) for gain in row[gain_field]))
            row[minimum_field] = min(row[gain_field])
            row[maximum_field] = max(row[gain_field])
            row['ranking_score'] = score if objective == 'weighted_gain' else row[minimum_field]
            if gear in leaders:
                require(all(leaders[gear][field] == row[field] for field in
                    [weighted_field,value_field,gain_field]))
            else:
                leaders[gear] = row
            leaders = dict(sorted(leaders.items(), key=lambda item:
                (-item[1]['ranking_score'], item[0]) if objective == 'weighted_gain' else
                (-item[1]['ranking_score'], -item[1][weighted_field], item[0]))[:args.top])
        if not page_complete:
            break
        next_cursor = selected['resume_after_window_offset_decimal']
        if next_cursor is None:
            exhausted = True
            stop = 'all_saved_resolved_results_complete'
            break
        next_cursor = int(next_cursor)
        require(cursor is None or next_cursor > cursor)
        cursor = next_cursor
    attach_saved_loadouts(args.journal, leaders, source, deadline + 5)
    current = journal.journal_status(args.journal, deadline=deadline + 5)
    require(current == source and monotonic() <= deadline + 5)
    complete = exhausted and completed == source['window_count'] and completed > 0
    report = {'mode': 'saved_journal_simulation', 'journal_coverage': source,
        'encounter_contexts': [{'encounter_index':index, 'weight':weights[index-1],
            'engine_sha256':context['engine'], 'simulation_profile_sha256':context['profile'],
            'fingerprints':context['fingerprints'], 'encounter_settings':context['settings'],
            'baseline':baselines.get(index)} for index,context in sorted(contexts.items())],
        'unvisited_encounter_indices':[index for index in range(1,len(weights)+1) if index not in contexts],
        'all_saved_resolved_results_complete': complete, 'ranking_scope':
            'all_saved_resolved_requests' if complete else
            'completed_candidates_across_all_requested_encounters' if incomplete_page_candidates is not None
            else 'completed_pages_only',
        'incomplete_page_completed_candidate_count':incomplete_page_candidates,
        'ranking_objective':objective,
        'rank_by':metric,'ranking_score_unit':roles.UNITS[metric],
        'ranking_basis': ('role_metric_point_estimates_not_statistical_optimality' if metric!='dps' else
            'weighted_dps_point_estimates_not_statistical_optimality' if objective == 'weighted_gain'
            else 'minimum_encounter_dps_gain_point_estimates_not_statistical_optimality'),
        'encounter_weights': weights, 'baseline_simulator_gear_sha256': baseline_gear,
        'ranked_gear_sets': list(leaders.values()), 'top_limit': args.top,
        'visited_window_count': visited, 'completed_window_count': completed, 'visited_page_count': page_count,
        'page_progress': progress, 'page_progress_truncated': page_count > len(progress),
        'stop_reason': stop, 'restart_from_beginning_for_ranking': not complete,
        'new_runs': new_runs, 'new_iterations': new_iterations, 'checkpoint_hits': hits,
        'simulation_performed': new_runs > 0, 'global_ranking_verified': False,
        'best_in_slot_verified': False, 'feasible_set_coverage_verified': False, 'game_execution': False}
    require(len(encoded(report)) <= MAX_TOTAL_BYTES and monotonic() <= deadline + 5)
    return report


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'journal', 'template', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    cli.add_argument('--encounter-lookup', type=Path, action='append', required=True)
    cli.add_argument('--encounter-weight', type=float, action='append')
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--page-limit', type=int, default=8)
    cli.add_argument('--max-pages', type=int, default=64)
    cli.add_argument('--top', type=int, default=10)
    cli.add_argument('--ranking-objective',choices=RANKING_OBJECTIVES,default='weighted_gain',
        help='Rank by weighted average improvement or the minimum improvement across every supplied encounter')
    cli.add_argument('--rank-by',choices=roles.FIELDS,default='dps')
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--checkpoint-only', action='store_true')
    cli.add_argument('--max-new-runs', type=int, default=16)
    cli.add_argument('--iteration-budget', type=int, default=100000)
    cli.add_argument('--time-budget-seconds', type=int, default=180)
    cli.add_argument('--simulation-journal-mib', type=int, default=64)
    args = cli.parse_args(argv)
    try:
        print(encoded(asyncio.run(execute(args))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Saved journal execution interrupted; numeric checkpoints remain available.\n')
    except Exception:
        cli.exit(1, 'Saved journal execution stopped: invalid, changed, unsupported or over-budget local inputs; numeric checkpoints remain available.\n')


if __name__ == '__main__':
    main()
