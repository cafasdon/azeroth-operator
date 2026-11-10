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
from tools import encounter_frontier
from tools import encounter_compromise

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


def saved_set_projection(value):
    """Validate the optional, numeric catalogue evidence before reporting it."""
    before_count = 'before_uncatalogued_equipped_slots'
    after_count = 'projected_uncatalogued_equipped_slots'
    require(type(value) is dict and set(value) == {
        'changes', before_count, after_count, 'native_set_bonuses_verified'})
    require(value['native_set_bonuses_verified'] is False)
    changes = value['changes']
    require(type(changes) is list and len(changes) <= 38)
    seen_sets = []
    for change in changes:
        before_ids = 'before_distinct_item_ids'
        after_ids = 'projected_distinct_item_ids'
        require(type(change) is dict and set(change) == {'set_id', before_ids, after_ids})
        seen_sets.append(integer(change['set_id'], 1, 2**64 - 1))
        for field in (before_ids, after_ids):
            ids = change[field]
            require(type(ids) is list and len(ids) <= 19)
            require(ids == sorted({integer(item, 1, 2**64 - 1) for item in ids}))
        require(change[before_ids] != change[after_ids])
    require(seen_sets == sorted(set(seen_sets)))
    integer(value[before_count], 0, 19)
    integer(value[after_count], 0, 19)
    return value


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
            bank_retrievals = [
                selection for selection in selections
                if selection['source']['kind'] in ('character_bank', 'account_bank')
            ]
            bank_retrievals.sort(key=lambda selection: (
                selection['source']['kind'], selection['source']['container'],
                selection['source']['slot'], selection['target_slot']))
            leader['saved_loadout'] = {'basis':'validated_saved_plan',
                'snapshot_id':summary['snapshot_id'],'source_report_sha256':row[3],
                'simulator_equipment_spec':spec,'selections':selections,
                'bank_retrievals':bank_retrievals,
                'cleared_equipment_slots':cleared,
                'current_locations_verified':False,'native_equip_eligibility_verified':False,
                'game_execution':False}
            if draft.get('explicit_clear_equipment_slots'):
                leader['saved_loadout']['explicit_clear_equipment_slots'] = draft['explicit_clear_equipment_slots']
            if 'projected_set_changes' in draft:
                leader['saved_loadout']['projected_set_changes'] = saved_set_projection(draft['projected_set_changes'])
    finally:
        connection.close()


def encounter_improvement_floor(args):
    """Read an optional finite bound in the selected role's improvement units."""
    return pages.encounters.encounter_improvement_floor(args)


def retain_encounter_leaders(leaders, row, gain_field):
    """Keep one point-estimate maximum per encounter; gains already use role direction."""
    gear = row['simulator_gear_sha256']
    for index, gain in enumerate(row[gain_field], 1):
        previous = leaders.get(index)
        if previous is None or (-gain, gear) < (
                -previous[gain_field][index-1], previous['simulator_gear_sha256']):
            leaders[index] = row


async def execute(args, *, deadline=None):
    include_frontier, frontier_offset = encounter_frontier.options(args)
    include_compromise, compromise_offset = encounter_compromise.options(args)
    include_encounter_leaders = getattr(args, 'include_encounter_leaders', False)
    require(type(include_encounter_leaders) is bool)
    encounter_leaders = {}
    objective = ranking_objective(args)
    floor = encounter_improvement_floor(args)
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
    frontier = encounter_frontier.Frontier(len(weights), gain_field, deadline=deadline) if include_frontier else None
    compromise = encounter_compromise.Compromise(len(weights), gain_field, deadline=deadline) if include_compromise else None
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
    floor_excluded = 0
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
                'fingerprints': result['fingerprints'], 'settings': result['encounter_settings'],
                'hunter_quiver_observation':result.get('hunter_quiver_observation')}
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
            require(ranking.get('minimum_encounter_improvement') == floor)
            floor_excluded += ranking.get('encounter_floor_excluded_candidate_instances',0)
        else:
            completed += count
            if comparison['weighted_status'] != 'complete_for_selected_resolved_requests':
                stop = comparison['weighted_status']
                break
            ranking = comparison['weighted_ranking'] if metric=='dps' else comparison['metric_ranking']
        first = {row['review']: row for row in profiles[0]['result']['results']}
        metrics = [{row['review']: row for row in profile['result']['results']} for profile in profiles]
        candidates = {row['review']: row for row in ranking['results']}
        if include_encounter_leaders and not page_complete:
            candidates.update({row['review']: row for row in ranking['encounter_leader_candidates']})
        for weighted in candidates.values():
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
            if floor is not None and row[minimum_field] < floor:
                floor_excluded += 1
                continue
            row['ranking_score'] = score if objective == 'weighted_gain' else row[minimum_field]
            if frontier is not None and page_complete:
                frontier.add(row)
            if compromise is not None and page_complete:
                compromise.add(row)
            if include_encounter_leaders:
                retain_encounter_leaders(encounter_leaders, row, gain_field)
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
    frontier_report = frontier.report(complete=exhausted and completed == source['window_count'] and completed > 0,
        offset=frontier_offset, limit=args.top) if frontier is not None else None
    compromise_report = compromise.report(complete=exhausted and completed == source['window_count'] and completed > 0,
        offset=compromise_offset, limit=args.top) if compromise is not None else None
    if compromise_report is not None:
        attach_saved_loadouts(args.journal,
            {row['gear_set']['simulator_gear_sha256']: row['gear_set'] for row in compromise_report['gear_sets']},
            source, deadline + 5)
    if frontier_report is not None:
        attach_saved_loadouts(args.journal,
            {row['simulator_gear_sha256']: row for row in frontier_report['gear_sets']}, source, deadline + 5)
    attach_saved_loadouts(args.journal, leaders, source, deadline + 5)
    if include_encounter_leaders:
        # At most eight distinct sets; preserve the existing bounded join.
        attach_saved_loadouts(args.journal,
            {row['simulator_gear_sha256']: row for row in encounter_leaders.values()}, source, deadline + 5)
    current = journal.journal_status(args.journal, deadline=deadline + 5)
    require(current == source and monotonic() <= deadline + 5)
    complete = exhausted and completed == source['window_count'] and completed > 0
    report = {'mode': 'saved_journal_simulation', 'journal_coverage': source,
        'encounter_contexts': [{'encounter_index':index, 'weight':weights[index-1],
            'engine_sha256':context['engine'], 'simulation_profile_sha256':context['profile'],
            'fingerprints':context['fingerprints'], 'encounter_settings':context['settings'],
            'hunter_quiver_observation':context['hunter_quiver_observation'],
            'baseline':baselines.get(index)} for index,context in sorted(contexts.items())],
        'unvisited_encounter_indices':[index for index in range(1,len(weights)+1) if index not in contexts],
        'all_saved_resolved_results_complete': complete, 'ranking_scope':
            'all_saved_resolved_requests' if complete else
            'completed_candidates_across_all_requested_encounters' if incomplete_page_candidates is not None
            else 'completed_pages_only',
        'incomplete_page_completed_candidate_count':incomplete_page_candidates,
        'ranking_objective':objective,
        'minimum_encounter_improvement':floor,
        'encounter_floor_excluded_candidate_instances':floor_excluded,
        'encounter_floor_basis':'all_requested_encounter_point_estimates_not_confidence_bounds',
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
    if frontier_report is not None:
        frontier_report.update(ranking_scope=report['ranking_scope'], rank_by=metric,
            improvement_unit=roles.UNITS[metric], minimum_encounter_improvement=floor,
            all_saved_resolved_results_complete=complete)
        report['encounter_frontier'] = frontier_report
    if compromise_report is not None:
        compromise_report.update(ranking_scope=report['ranking_scope'], rank_by=metric,
            improvement_unit=roles.UNITS[metric], minimum_encounter_improvement=floor,
            all_saved_resolved_results_complete=complete)
        report['encounter_compromise'] = compromise_report
    if include_encounter_leaders:
        report['encounter_leaders'] = {
            'basis': 'completed_candidate_point_estimates_after_encounter_floor',
            'ranking_scope': report['ranking_scope'],
            'all_saved_resolved_results_complete': complete,
            'tie_break': 'simulator_gear_sha256_ascending',
            'statistical_superiority_verified': False,
            'leaders': [{'encounter_index': index,
                'improvement': encounter_leaders[index][gain_field][index-1],
                'gear_set': encounter_leaders[index]} for index in sorted(encounter_leaders)],
            'encounters_without_candidate': [index for index in range(1,len(weights)+1)
                if index not in encounter_leaders]}
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
    cli.add_argument('--minimum-encounter-improvement',type=float,
        help='Require at least this improvement in every requested encounter before top selection; uses ranking_score_unit, including the baseline')
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--include-encounter-leaders', action='store_true',
        help='Retain the best completed point-estimate set per encounter after the same improvement floor, without extra simulations')
    cli.add_argument('--include-encounter-compromise', action='store_true',
        help='Page saved gear by minimum worst gap from encounter-specific best eligible improvements; no extra simulations')
    cli.add_argument('--encounter-compromise-offset', type=int, default=0)
    cli.add_argument('--include-encounter-frontier', action='store_true',
        help='Page non-dominated gear across complete saved encounter results; point estimates only, no extra simulations')
    cli.add_argument('--encounter-frontier-offset', type=int, default=0,
        help='Skip this many frontier sets in gear-hash order (requires --include-encounter-frontier)')
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
