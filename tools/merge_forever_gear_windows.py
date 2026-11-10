"""Aggregate bounded local plan-window artifacts; never infer a simulated optimum."""
import argparse
from collections import Counter
import hashlib
import json
from math import isfinite
from pathlib import Path

from tools.rank_forever_slot import SIMULATOR_SLOTS, validate_candidate_scan_limit
from tools import positional_equipment as layout

MAX_REPORT_BYTES = 4 * 1024 * 1024
MAX_TOTAL_BYTES = 16 * 1024 * 1024
ACCOUNTING_FIELDS = ('baseline_coordinates', 'pre_review_excluded_coordinates',
                    'proposed_alternative_coordinates', 'resolved_alternative_draft_instances',
                    'equivalent_alternative_coordinates', 'unresolved_proposed_coordinates',
                    'baseline_review_instances')
NATIVE_EQUIPMENT_SLOTS = (1, 2, 3, 15, 5, 9, 10, 6, 7, 8, 11, 12, 13, 14, 16, 17, 18)


def empty_accounting():
    return {'basis': 'validated_local_plan_report_metadata',
            **{field: 0 for field in ACCOUNTING_FIELDS}}


def require(condition):
    if not condition:
        raise ValueError('Invalid or incompatible gear window report')


def integer(value, minimum=0, maximum=None):
    require(type(value) is int and value >= minimum and (maximum is None or value <= maximum))
    return value


def digest(value):
    require(isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value))
    return value


def checked_inventory_evidence(value):
    """Validate reported inventory scope; observed sections never prove ownership."""
    if value is None:
        return None
    fields={'scope','observed_storage_sections','unavailable_storage_sections','all_storage_sections_observed',
        'catalogue_unmatched_item_locations','catalogue_unmatched_distinct_item_ids','catalogue_unmatched_item_ids',
        'catalogue_unmatched_item_ids_truncated','complete_ownership_verified',
        'complete_simulator_resolution_verified','best_in_slot_verified'}
    require(isinstance(value,dict) and set(value)==fields)
    require(value['scope']=='all_observed_storage_before_filters')
    storages=['equipment','bags','character_bank','account_bank']
    observed=value['observed_storage_sections']; missing=value['unavailable_storage_sections']
    for group in [observed,missing]:
        require(isinstance(group,list) and len(group)<=4
            and all(isinstance(name,str) and name in storages for name in group)
            and group==[name for name in storages if name in group])
    require(set(observed).isdisjoint(missing) and set(observed+missing)==set(storages))
    require(value['all_storage_sections_observed'] is (not missing))
    locations=integer(value['catalogue_unmatched_item_locations'],0,2**64-1)
    count=integer(value['catalogue_unmatched_distinct_item_ids'],0,min(locations,2**32-1))
    require((count==0)==(locations==0) and (bool(observed) or locations==0))
    ids=value['catalogue_unmatched_item_ids']
    require(isinstance(ids,list) and len(ids)==min(50,count))
    for item in ids: integer(item,1,2**32-1)
    require(ids==sorted(set(ids)))
    require(value['catalogue_unmatched_item_ids_truncated'] is (count>50))
    require(all(value[flag] is False for flag in ['complete_ownership_verified',
        'complete_simulator_resolution_verified','best_in_slot_verified']))
    return value


def retained_equipment(row):
    if 'simulator_equipment_spec' not in row:
        return None
    spec = row['simulator_equipment_spec']
    require(isinstance(spec, dict) and set(spec) in ({'items'},{'items','preserveSlots'}))
    layout.positional(spec)
    require(isinstance(spec['items'], list) and len(spec['items']) == 17)
    for item in spec['items']:
        require(isinstance(item, dict))
        if item:
            require(set(item) == {'id', 'enchant', 'randomSuffix'})
            integer(item['id'], 1, 2**31 - 1)
            integer(item['enchant'], 0, 2**31 - 1)
            integer(item['randomSuffix'], -(2**31), 2**31 - 1)
    require(hashlib.sha256(json.dumps(spec, sort_keys=True, separators=(',', ':'),
                                    allow_nan=False).encode()).hexdigest() == row['simulator_gear_sha256'])
    return spec


def selections_for_review(rows):
    require(isinstance(rows, list) and len(rows) <= 17)
    for row in rows:
        require(isinstance(row, dict) and set(row) == {'target_slot', 'source', 'expected_item_id'})
        integer(row['target_slot'], 1, 19)
        integer(row['expected_item_id'], 1, 2**32 - 1)
        source = row['source']
        require(isinstance(source, dict))
        kind = source.get('kind')
        fields = {'equipment': {'slot'}, 'bag': {'bag', 'slot'},
                  'character_bank': {'container', 'slot'}, 'account_bank': {'container', 'slot'}}
        require(isinstance(kind, str) and kind in fields and set(source) == {'kind', *fields[kind]})
        for field in fields[kind]:
            integer(source[field], 1 if field == 'slot' else 0, 2**32 - 1)
    return rows


def native_clear_slots(value):
    """Require the canonical native-slot list, without coercion or duplicates."""
    require(isinstance(value, list) and 1 <= len(value) <= 17)
    require(all(type(slot) is int and slot in NATIVE_EQUIPMENT_SLOTS for slot in value))
    require(value == sorted(set(value)))
    return value


def cleared_for_review(draft, search=None):
    """Validate explicit holes separately from the legacy automatic off-hand clear."""
    cleared = draft.get('cleared_equipment_slots', [])
    if 'explicit_clear_equipment_slots' not in draft:
        require(isinstance(cleared, list) and (not cleared or
            (len(cleared) == 1 and type(cleared[0]) is int and cleared[0] == 17)))
        if not cleared:
            return cleared
        explicit = []
    else:
        explicit = native_clear_slots(draft['explicit_clear_equipment_slots'])
        require(type(draft.get('review')) is int and draft['review'] > 1)
        if search is not None:
            require(search.get('equipment_layout') == layout.CONTRACT
                    and search.get('include_empty_slots') is True)
            requested = search.get('requested_slots')
            choices = search.get('empty_choices_by_slot')
            require(isinstance(requested, list) and isinstance(choices, dict))
            for slot in explicit:
                name = SIMULATOR_SLOTS[NATIVE_EQUIPMENT_SLOTS.index(slot)]
                require(name in requested and choices.get(name) is True)
        native_clear_slots(cleared)
    selected = selections_for_review(draft['selections'])
    targets = {row['target_slot'] for row in selected}
    spec = retained_equipment(draft)
    require(spec is not None)
    if explicit:
        require(layout.positional(spec) and targets.isdisjoint(explicit))
        require(targets.issubset(NATIVE_EQUIPMENT_SLOTS) and len(targets) == len(selected))
        sources = [json.dumps(row['source'], sort_keys=True) for row in selected]
        require(len(set(sources)) == len(sources))
        require(all(spec['items'][NATIVE_EQUIPMENT_SLOTS.index(slot)] == {} for slot in explicit))
        require(set(cleared) in (set(explicit), set(explicit) | {17}))
    if not explicit or 17 in set(cleared) - set(explicit):
        require(16 in targets and 17 not in targets and spec['items'][15] == {})
    return cleared


def coordinate_outcomes(report):
    window = report['gear_search']['combination_window']
    if 'coordinate_outcomes' not in window:
        return None
    rows = window['coordinate_outcomes']
    require(isinstance(rows, list) and len(rows) == window['considered'])
    counts = {status: 0 for status in ['baseline', 'excluded', 'resolved', 'equivalent', 'unresolved']}
    latest_review = 1
    for coordinate, row in enumerate(rows, window['offset']):
        require(isinstance(row, dict) and row.get('coordinate_decimal') == str(coordinate))
        status = row.get('status')
        require(isinstance(status, str) and status in counts)
        counts[status] += 1
        if status in ('baseline', 'resolved', 'equivalent'):
            require(set(row) == {'coordinate_decimal', 'status', 'review'})
            review = integer(row['review'], 1, len(report['resolved_drafts']))
            if status == 'baseline':
                require(coordinate == 0 and review == 1)
            elif status == 'resolved':
                require(review == latest_review + 1)
                latest_review = review
            else:
                require(review <= latest_review)
        else:
            require(set(row) == {'coordinate_decimal', 'status', 'reason'})
            reasons = ('would_vacate_another_slot', 'reused_source_location') if status == 'excluded' else (
                'native_slot_category_conflict', 'native_required_level_shortfall',
                'catalogue_requirement_conflict', 'catalogue_unique_item_duplicate',
                'catalogue_shared_limit_category', 'unresolved_simulator_draft')
            require(isinstance(row['reason'], str) and row['reason'] in reasons)
    search = report['gear_search']
    proposed = search['proposed_alternative_drafts']
    resolved = len(report['resolved_drafts']) - 1
    baseline = int(window['offset'] == 0 and window['considered'] > 0)
    require(counts == {'baseline': baseline, 'excluded': window['considered'] - baseline - proposed,
                       'resolved': resolved, 'equivalent': search['equivalent_simulator_drafts'],
                       'unresolved': proposed - resolved - search['equivalent_simulator_drafts']})
    return rows


def skip_reason_coverage(reasons, accounting):
    skipped = (accounting['pre_review_excluded_coordinates']
               + accounting['unresolved_proposed_coordinates'])
    return {'counts':dict(sorted(reasons.items())),
            'skipped_coordinates_without_reason':skipped - sum(reasons.values())}


def merge_reports(reports):
    require(isinstance(reports, list) and 1 <= len(reports) <= 64)
    signature = baseline = scope = snapshot = binding = None
    intervals, windows, gear, equipment = [], [], {}, {}
    accounting = empty_accounting()
    reasons = Counter()
    for index, report in enumerate(reports, 1):
        require(isinstance(report, dict) and report.get('mode') == 'plan_only'
                and report.get('best_in_slot_verified') is False)
        require(integer(report.get('baseline_review'), 1, 1) == 1)
        search = report.get('gear_search')
        require(isinstance(search, dict))
        inventory = checked_inventory_evidence(search.get('inventory_evidence'))
        current_snapshot = digest(search.get('snapshot_id'))
        slots = search.get('requested_slots')
        require(isinstance(slots, list) and 1 <= len(slots) <= 17
                and all(isinstance(slot, str) and slot in SIMULATOR_SLOTS for slot in slots)
                and len(set(slots)) == len(slots))
        mode = search.get('equipment_layout', 'auto_place')
        require(mode in ('auto_place', layout.CONTRACT))
        include_empty = search.get('include_empty_slots', False)
        require(type(include_empty) is bool and (not include_empty or mode == layout.CONTRACT))
        empty_choices = search.get('empty_choices_by_slot')
        if include_empty:
            require(isinstance(empty_choices, dict) and set(empty_choices) == set(slots)
                    and all(type(value) is bool for value in empty_choices.values()))
        else:
            require('empty_choices_by_slot' not in search)
        counts = search.get('candidate_locations_by_slot')
        require(isinstance(counts, dict) and set(counts) == set(slots))
        scan_limit = validate_candidate_scan_limit(search.get('candidate_scan_limit', 256))
        unresolved_candidates = search.get('unresolved_candidate_locations_by_slot')
        if 'unresolved_candidate_locations_by_slot' in search:
            require(isinstance(unresolved_candidates, dict) and set(unresolved_candidates) == set(slots))
        swaps = integer(search.get('maximum_simultaneous_swaps'), 1, 17)
        coefficients = [1] + [0] * min(swaps, len(slots))
        for slot in slots:
            count = integer(counts[slot], 0, scan_limit)
            if unresolved_candidates is not None:
                integer(unresolved_candidates[slot], 0, scan_limit - count)
            if include_empty:
                count += int(empty_choices[slot])
            for size in range(len(coefficients) - 1, 0, -1):
                coefficients[size] += coefficients[size - 1] * count
        total = sum(coefficients)
        require(integer(search.get('combinations_including_baseline'), 1) == total)
        require(isinstance(search.get('candidate_filters'), dict))
        window = search.get('combination_window')
        require(isinstance(window, dict) and integer(window.get('version'), 1, 1) == 1)
        current_scope = digest(window.get('scope_sha256'))
        current_binding = report.get('collection_binding_sha256')
        if current_binding is not None:
            digest(current_binding)
        offset = integer(window.get('offset'))
        limit = integer(window.get('limit'), 1, 1024)
        considered = integer(window.get('considered'), 0, limit)
        require(considered == min(limit, max(0, total - offset)))
        require(window.get('total_combinations_decimal') == str(total))
        end = offset + considered
        next_offset = end if end < total else None
        if next_offset is None:
            require(window.get('next_offset') is None)
        else:
            require(integer(window.get('next_offset')) == next_offset)
        require(window.get('baseline_reused') is True)
        require(window.get('covers_requested_combinations') is (offset == 0 and considered == total))
        drafts = report.get('resolved_drafts')
        require(isinstance(drafts, list) and 1 <= len(drafts) <= 129)
        require(all(isinstance(row, dict) for row in drafts))
        retains_equipment = 'simulator_equipment_spec' in drafts[0]
        require(all(('simulator_equipment_spec' in row) == retains_equipment for row in drafts))
        require(not include_empty or retains_equipment)
        if retains_equipment:
            require(all(layout.positional(retained_equipment(row)) == (mode==layout.CONTRACT) for row in drafts))
        if include_empty:
            baseline_spec = retained_equipment(drafts[0])
            require(all(empty_choices[slot] is bool(baseline_spec['items'][SIMULATOR_SLOTS.index(slot)])
                for slot in slots))
        current_signature = json.dumps([current_snapshot, current_scope, current_binding, retains_equipment, slots, counts, swaps,
            search['candidate_filters'], search.get('inventory_evidence'),mode,include_empty,empty_choices,
            unresolved_candidates],
            sort_keys=True, separators=(',', ':'), allow_nan=False)
        if signature is None:
            signature, scope, snapshot = current_signature, current_scope, current_snapshot
            binding = current_binding
        require(signature == current_signature)
        require(integer(search.get('resolved_drafts_including_baseline'), 1, 129) == len(drafts))
        proposed = integer(search.get('proposed_alternative_drafts'), 0,
                           considered - (1 if offset == 0 and considered else 0))
        equivalent = integer(search.get('equivalent_simulator_drafts'), 0, proposed)
        require(len(drafts) - 1 + equivalent <= proposed)
        baseline_coordinates = int(offset == 0 and considered > 0)
        counts_for_window = (baseline_coordinates, considered - baseline_coordinates - proposed,
                             proposed, len(drafts) - 1, equivalent,
                             proposed - (len(drafts) - 1) - equivalent, 1)
        for field, value in zip(ACCOUNTING_FIELDS, counts_for_window, strict=True):
            accounting[field] += value
        keys = []
        for review, row in enumerate(drafts, 1):
            require(isinstance(row, dict) and integer(row.get('review'), 1, 129) == review)
            selections = row.get('selections')
            explicit = 'explicit_clear_equipment_slots' in row
            require(isinstance(selections, list) and (not selections if review == 1
                    else (0 if explicit else 1) <= len(selections) <= 17))
            if explicit:
                cleared_for_review(row, search)
            key = digest(row.get('simulator_gear_sha256'))
            spec = retained_equipment(row)
            if spec is not None:
                require(key not in equipment or equipment[key] == spec)
                equipment[key] = spec
            keys.append(key)
            gear.setdefault(key, []).append({'window_index': index, 'review': review})
        require(integer(search.get('distinct_simulator_gear_sets'), 1, len(drafts)) == len(set(keys)))
        if baseline is None:
            baseline = keys[0]
        require(baseline == keys[0])
        outcomes = coordinate_outcomes(report)
        reasons.update(row['reason'] for row in outcomes or () if 'reason' in row)
        if considered:
            intervals.append((offset, end))
        windows.append({'window_index': index, 'offset_decimal': str(offset), 'considered': considered})
    cursor, gaps = 0, []
    for start, end in sorted(intervals):
        require(start >= cursor)  # Reject overlapping or duplicate coordinate ranges.
        if start > cursor:
            gaps.append({'start_decimal': str(cursor), 'end_decimal': str(start)})
        cursor = end
    if cursor < total:
        gaps.append({'start_decimal': str(cursor), 'end_decimal': str(total)})
    ordered = [baseline, *sorted(key for key in gear if key != baseline)]
    return {'mode': 'gear_window_coverage', 'basis': 'validated_local_plan_report_metadata',
            **({'collection_binding_sha256': binding} if binding is not None else {}),
            'snapshot_id': snapshot, 'scope_sha256': scope, 'window_count': len(windows),
            'inventory_evidence':inventory,
            'unresolved_candidate_locations_by_slot': unresolved_candidates,
            'resolution_accounting': accounting,
            'skip_reason_coverage': skip_reason_coverage(reasons, accounting),
            'total_combinations_decimal': str(total), 'covered_coordinate_count': sum(end - start for start, end in intervals),
            'reported_coordinate_coverage_complete': not gaps, 'uncovered_ranges': gaps,
            'next_uncovered_offset_decimal': gaps[0]['start_decimal'] if gaps else None,
            'windows': windows, 'distinct_simulator_gear_sets': len(ordered), 'baseline_global_review': 1,
            'equipment_specs_retained': retains_equipment,
            'drafts': [{'global_review': i, 'simulator_gear_sha256': key, 'references': gear[key]}
                       | ({'simulator_equipment_spec': equipment[key]} if retains_equipment else {})
                       for i, key in enumerate(ordered, 1)],
            'feasible_set_coverage_verified': False, 'best_in_slot_verified': False,
            'simulation_performed': False, 'game_execution': False}


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def reject_constant(_):
    raise ValueError('Nonfinite JSON constant')


def finite_number(token):
    value = float(token)
    require(isfinite(value))
    return value


def parse_report(data):
    require(len(data) <= MAX_REPORT_BYTES)
    return json.loads(data, object_pairs_hook=unique_fields,
                      parse_constant=reject_constant, parse_float=finite_number)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', nargs='+', type=Path)
    args = parser.parse_args(argv)
    try:
        require(1 <= len(args.reports) <= 64)
        reports, size = [], 0
        for path in args.reports:
            with path.open('rb') as handle:
                data = handle.read(MAX_REPORT_BYTES + 1)
            size += len(data)
            require(len(data) <= MAX_REPORT_BYTES and size <= MAX_TOTAL_BYTES)
            reports.append(parse_report(data))
        print(json.dumps(merge_reports(reports), allow_nan=False))
    except Exception:
        parser.exit(1, 'Gear window aggregation rejected: invalid, incompatible or oversized plan reports.\n')


if __name__ == '__main__':
    main()
