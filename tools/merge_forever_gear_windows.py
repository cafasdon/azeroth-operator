"""Aggregate bounded local plan-window artifacts; never infer a simulated optimum."""
import argparse
import json
from math import isfinite
from pathlib import Path

from tools.rank_forever_slot import SIMULATOR_SLOTS

MAX_REPORT_BYTES = 4 * 1024 * 1024
MAX_TOTAL_BYTES = 16 * 1024 * 1024


def require(condition):
    if not condition:
        raise ValueError('Invalid or incompatible gear window report')


def integer(value, minimum=0, maximum=None):
    require(type(value) is int and value >= minimum and (maximum is None or value <= maximum))
    return value


def digest(value):
    require(isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value))
    return value


def merge_reports(reports):
    require(isinstance(reports, list) and 1 <= len(reports) <= 64)
    signature = baseline = scope = snapshot = None
    intervals, windows, gear = [], [], {}
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
        current_signature = json.dumps([current_snapshot, current_scope, slots, counts, swaps,
            search['candidate_filters'], search.get('inventory_evidence')],
            sort_keys=True, separators=(',', ':'), allow_nan=False)
        if signature is None:
            signature, scope, snapshot = current_signature, current_scope, current_snapshot
        require(signature == current_signature)
        drafts = report.get('resolved_drafts')
        require(isinstance(drafts, list) and 1 <= len(drafts) <= 129)
        require(integer(search.get('resolved_drafts_including_baseline'), 1, 129) == len(drafts))
        proposed = integer(search.get('proposed_alternative_drafts'), 0,
                           considered - (1 if offset == 0 and considered else 0))
        equivalent = integer(search.get('equivalent_simulator_drafts'), 0, proposed)
        require(len(drafts) - 1 + equivalent <= proposed)
        keys = []
        for review, row in enumerate(drafts, 1):
            require(isinstance(row, dict) and integer(row.get('review'), 1, 129) == review)
            selections = row.get('selections')
            require(isinstance(selections, list) and (not selections if review == 1 else 1 <= len(selections) <= 17))
            key = digest(row.get('simulator_gear_sha256'))
            keys.append(key)
            gear.setdefault(key, []).append({'window_index': index, 'review': review})
        require(integer(search.get('distinct_simulator_gear_sets'), 1, len(drafts)) == len(set(keys)))
        if baseline is None:
            baseline = keys[0]
        require(baseline == keys[0])
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
            'snapshot_id': snapshot, 'scope_sha256': scope, 'window_count': len(windows),
            'total_combinations_decimal': str(total), 'covered_coordinate_count': sum(end - start for start, end in intervals),
            'reported_coordinate_coverage_complete': not gaps, 'uncovered_ranges': gaps,
            'next_uncovered_offset_decimal': gaps[0]['start_decimal'] if gaps else None,
            'windows': windows, 'distinct_simulator_gear_sets': len(ordered), 'baseline_global_review': 1,
            'drafts': [{'global_review': i, 'simulator_gear_sha256': key, 'references': gear[key]}
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
