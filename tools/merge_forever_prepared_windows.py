"""Group compatible prepared requests against saved journal rows; never execute."""
import argparse
from copy import deepcopy
import hashlib
from pathlib import Path
from time import monotonic

from tools import journal_forever_gear_windows as journal
from tools.merge_forever_gear_windows import (
    MAX_REPORT_BYTES, MAX_TOTAL_BYTES, digest, integer, merge_reports, parse_report, require,
)
from tools.prepare_forever_gear_window import encoded
from tools.run_forever_sim import single_player, hunter_quiver_observation

FINGERPRINTS = ('mcp_sha256', 'template_sha256', 'encounter_sha256',
                'preparer_sha256', 'adapter_sha256', 'catalogue_sha256', 'layout_sha256',
                'talent_identity_sha256', 'talent_map_sha256')


def merge_prepared(path, artifacts):
    require(isinstance(artifacts, list) and 1 <= len(artifacts) <= 64)
    sizes = [len(encoded(artifact)) for artifact in artifacts]
    require(all(size <= MAX_REPORT_BYTES for size in sizes) and sum(sizes) <= MAX_TOTAL_BYTES)
    connection, deadline = journal.connect(path)
    reports, preparations, requests = [], [], {}
    fingerprints = context_bytes = None
    bag_items = None
    first_observation = True
    quiver_observation = None
    instances = source_bytes = 0
    try:
        for index, artifact in enumerate(artifacts, 1):
            require(monotonic() <= deadline)
            require(isinstance(artifact, dict) and artifact.get('mode') == 'prepared_gear_window_requests')
            require(all(artifact.get(flag) is False for flag in ['simulation_performed', 'game_execution',
                'best_in_slot_verified', 'feasible_set_coverage_verified']))
            current_fingerprints = {field: digest(artifact.get(field)) for field in FINGERPRINTS}
            if fingerprints is None: fingerprints = current_fingerprints
            require(current_fingerprints == fingerprints)
            offset = artifact.get('window_offset_decimal')
            require(isinstance(offset, str) and 1 <= len(offset) <= 64 and offset.isascii()
                    and offset.isdecimal() and str(int(offset)) == offset)
            row = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows WHERE start_key=?',
                                     (journal.coordinate(int(offset)),)).fetchone()
            require(row is not None)
            report, summary = journal.checked_row(row)
            source_bytes += len(row[2])
            require(source_bytes <= MAX_TOTAL_BYTES)
            require(summary['equipment_specs_retained'] and artifact.get('source_report_sha256') == row[3]
                    and artifact.get('snapshot_id') == summary['snapshot_id']
                    and artifact.get('scope_sha256') == summary['scope_sha256']
                    and encoded(artifact.get('resolution_accounting')) == encoded(summary['resolution_accounting']))
            rows = artifact.get('requests')
            require(isinstance(rows, list) and len(rows) == len(report['resolved_drafts']))
            for review, (prepared, draft) in enumerate(zip(rows, report['resolved_drafts'], strict=True), 1):
                require(monotonic() <= deadline)
                require(isinstance(prepared, dict) and set(prepared) == {
                    'review', 'simulator_gear_sha256', 'request_sha256', 'request'})
                require(integer(prepared['review'], 1, 129) == review
                        and prepared['simulator_gear_sha256'] == draft['simulator_gear_sha256'])
                request = prepared['request']
                require(isinstance(request, dict) and set(request) == {'raid', 'encounter', 'simOptions', 'type'}
                        and request['type'] == 'SimTypeIndividual')
                observed = artifact.get('captured_equipped_bag_items')
                quiver_observation = hunter_quiver_observation(single_player(request['raid']), observed)
                require(first_observation or observed == bag_items)
                bag_items = observed
                first_observation = False
                data = encoded(request)
                require(hashlib.sha256(data).hexdigest() == digest(prepared['request_sha256']))
                context = deepcopy(request)
                equipment = single_player(context['raid']).pop('equipment')
                require(encoded(equipment) == encoded(draft['simulator_equipment_spec']))
                current_context = encoded(context)
                if context_bytes is None: context_bytes = current_context
                require(current_context == context_bytes)
                key = draft['simulator_gear_sha256']
                require(key not in requests or encoded(requests[key]['request']) == data)
                requests.setdefault(key, {'simulator_gear_sha256': key, 'request_sha256': prepared['request_sha256'],
                                          'request': request})
                instances += 1
            reports.append(report)
            preparations.append({'window_index': index, 'offset_decimal': offset, 'source_report_sha256': row[3],
                'prepared_artifact_sha256': hashlib.sha256(encoded(artifact)).hexdigest()})
        coverage = merge_reports(reports)
        jobs = [{**requests[draft['simulator_gear_sha256']], 'global_review': draft['global_review'],
                 'references': draft['references']} for draft in coverage.pop('drafts')]
        result = {'mode': 'prepared_gear_request_set', 'basis': 'journal_and_prepared_artifact_consistency',
            'hunter_quiver_observation':quiver_observation,
            'fingerprints': fingerprints, 'context_sha256': hashlib.sha256(context_bytes).hexdigest(),
            'search_coverage': coverage, 'preparations': preparations,
            'request_instances': instances, 'distinct_requests': len(jobs),
            'reused_request_instances': instances - len(jobs), 'requests': jobs,
            'simulation_performed': False, 'game_execution': False,
            'best_in_slot_verified': False, 'feasible_set_coverage_verified': False}
        require(monotonic() <= deadline and len(encoded(result)) <= MAX_TOTAL_BYTES)
        return result
    finally:
        connection.close()


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('--journal', type=Path, required=True)
    cli.add_argument('prepared_windows', type=Path, nargs='+')
    args = cli.parse_args(argv)
    try:
        require(1 <= len(args.prepared_windows) <= 64)
        artifacts, size = [], 0
        for path in args.prepared_windows:
            with path.open('rb') as handle: data = handle.read(MAX_REPORT_BYTES + 1)
            size += len(data)
            require(size <= MAX_TOTAL_BYTES)
            artifacts.append(parse_report(data))
        print(encoded(merge_prepared(args.journal, artifacts)).decode())
    except Exception:
        cli.exit(1, 'Prepared request aggregation rejected: invalid, incompatible or over-budget local data.\n')


if __name__ == '__main__':
    main()
