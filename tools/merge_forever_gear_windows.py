"""Aggregate bounded local plan-window artifacts; never infer a simulated optimum."""
import argparse
import hashlib
import json
from math import isfinite
from pathlib import Path

from tools.rank_forever_slot import SIMULATOR_SLOTS
from tools import positional_equipment as layout

MAX_REPORT_BYTES = 4 * 1024 * 1024
MAX_TOTAL_BYTES = 16 * 1024 * 1024
ACCOUNTING_FIELDS = ('baseline_coordinates', 'pre_review_excluded_coordinates',
                    'proposed_alternative_coordinates', 'resolved_alternative_draft_instances',
                    'equivalent_alternative_coordinates', 'unresolved_proposed_coordinates',
                    'baseline_review_instances')


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
                'catalogue_requirement_conflict', 'unresolved_simulator_draft')
            require(isinstance(row['reason'], str) and row['reason'] in reasons)
    search = report['gear_search']
    proposed = search['proposed_alternative_drafts']
    resolved = len(report['resolved_drafts']) - 1
    baseline = int(window['offset'] == 0 and window['considered'] > 0)
    require(counts == {'baseline': baseline, 'excluded': window['considered'] - baseline - proposed,
                       'resolved': resolved, 'equivalent': search['equivalent_simulator_drafts'],
                       'unresolved': proposed - resolved - search['equivalent_simulator_drafts']})
    return rows


def merge_reports(reports):
    require(isinstance(reports, list) and 1 <= len(reports) <= 64)
    signature = baseline = scope = snapshot = binding = None
    intervals, windows, gear, equipment = [], [], {}, {}
    accounting = empty_accounting()
    for index, report in enumerate(reports, 1):
        require(isinstance(report, dict) and report.get('mode') == 'plan_only'
                and report.get('best_in_slot_verified') is False)
        require(integer(report.get('baseline_review'), 1, 1) == 1)
        search = report.get('gear_search')
        require(isinstance(search, dict))
        current_snapshot = digest(search.get('snapshot_id'))
        slots = search.get('requested_slots')
        require(isinstance(slots, list) and 1 <= len(slots) <= 17
                and all(isinstance(slot, str) and slot in SIMULATOR_SLOTS for slot in slots)
                and len(set(slots)) == len(slots))
        counts = search.get('candidate_locations_by_slot')
        require(isinstance(counts, dict) and set(counts) == set(slots))
        swaps = integer(search.get('maximum_simultaneous_swaps'), 1, 17)
        coefficients = [1] + [0] * min(swaps, len(slots))
        for slot in slots:
            count = integer(counts[slot], 0, 256)
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
        mode=search.get('equipment_layout','auto_place')
        require(mode in ('auto_place',layout.CONTRACT))
        if retains_equipment:
            require(all(layout.positional(retained_equipment(row)) == (mode==layout.CONTRACT) for row in drafts))
        current_signature = json.dumps([current_snapshot, current_scope, current_binding, retains_equipment, slots, counts, swaps,
            search['candidate_filters'], search.get('inventory_evidence'),mode],
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
            require(isinstance(selections, list) and (not selections if review == 1 else 1 <= len(selections) <= 17))
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
        coordinate_outcomes(report)
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
            'resolution_accounting': accounting,
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
