"""Prepare bounded simulator requests from one saved window, never run them."""
import argparse
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
from time import monotonic

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools import journal_forever_gear_windows as journal
from tools import run_forever_sim as adapter
from tools import forever_talent_identity as talents
from tools.collect_forever_gear_windows import file_digest
from tools.merge_forever_gear_windows import (MAX_REPORT_BYTES, cleared_for_review, integer,
    parse_report, require, retained_equipment, selections_for_review)


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()


def read_input(path, expected_sha256):
    with path.open('rb') as handle:
        data = handle.read(MAX_REPORT_BYTES + 1)
    require(hashlib.sha256(data).hexdigest() == expected_sha256)
    return parse_report(data)


async def prepare(args):
    integer(args.window_offset)
    integer(args.time_budget_seconds, 1, 45)
    require(args.mcp.is_absolute() and args.mcp.is_file())
    args.mcp = args.mcp.resolve(strict=True)
    deadline = monotonic() + args.time_budget_seconds
    paths = [args.mcp, args.template.resolve(strict=True), args.encounter_lookup.resolve(strict=True),
             Path(__file__).resolve(), Path(adapter.__file__).resolve()]
    for key in ['AZEROTH_FOREVER_CATALOGUE']:
        raw = os.environ.get(key)
        require(raw is not None and Path(raw).is_absolute())
        paths.append(Path(raw).resolve(strict=True))
    paths.append(Path(adapter.layout.__file__).resolve())
    paths.extend([Path(talents.__file__).resolve(), talents.MAP_PATH.resolve(strict=True)])
    fingerprints = [file_digest(path, deadline) for path in paths]
    def unchanged():
        require([file_digest(path, deadline) for path in paths] == fingerprints)
    template = read_input(paths[1], fingerprints[1])
    encounter = read_input(paths[2], fingerprints[2])
    unchanged()
    adapter.check_consumable_groups(template)
    connection, _ = journal.connect(args.journal, deadline=deadline)
    try:
        row = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows WHERE start_key=?',
                                 (journal.coordinate(args.window_offset),)).fetchone()
        require(row is not None)
        report, summary = journal.checked_row(row)
        report_sha = row[3]
    finally:
        connection.close()
    require(summary['equipment_specs_retained'])
    for draft in report['resolved_drafts']:
        selections_for_review(draft['selections'])
        cleared_for_review(draft, report['gear_search'])
    requests, size = [], 0
    bag_items = None
    first_review = True
    with tempfile.TemporaryFile(mode='w+') as errlog:
        async with stdio_client(StdioServerParameters(command=str(args.mcp), env=dict(os.environ)), errlog=errlog) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                unchanged()
                for draft in report['resolved_drafts']:
                    selected = draft['selections']
                    targets = {row['target_slot'] for row in selected}
                    arguments = {'snapshot_id': summary['snapshot_id'], 'selections': selected}
                    if 'explicit_clear_equipment_slots' in draft:
                        arguments['clear_equipment_slots'] = draft['explicit_clear_equipment_slots']
                    if report['gear_search'].get('equipment_layout') == adapter.layout.CONTRACT:
                        arguments['positional_simulator_equipment']=True
                    if 16 in targets and 17 not in targets:
                        arguments['allow_two_hand_off_hand_clear'] = True
                    reply = await session.call_tool('review_equipment_selection', arguments)
                    require(not reply.isError and len(reply.content) == 1 and reply.content[0].type == 'text')
                    review = parse_report(reply.content[0].text.encode())
                    require(review.get('snapshot_id') == summary['snapshot_id']
                            and review.get('game_execution') is False and review.get('live') is False)
                    checks = review['checks']
                    require(cleared_for_review(draft, report['gear_search'])
                            == checks.get('simulator_cleared_equipment_slots', []))
                    require(all(checks.get(key) is True for key in ['selected_locations_observed',
                        'item_ids_match_snapshot', 'unique_sources_and_targets']))
                    spec = retained_equipment({'simulator_equipment_spec': checks.get('simulator_equipment_spec'),
                                              'simulator_gear_sha256': draft['simulator_gear_sha256']})
                    require(spec == draft['simulator_equipment_spec'])
                    request = adapter.request_from(template, review, encounter)
                    observed = review.get('captured_equipped_bag_items')
                    adapter.hunter_quiver_observation(adapter.single_player(template['raid']), observed)
                    require(first_review or observed == bag_items)
                    bag_items = observed
                    first_review = False
                    data = encoded(request)
                    size += len(data)
                    require(size <= MAX_REPORT_BYTES)
                    requests.append({'review': draft['review'], 'simulator_gear_sha256': draft['simulator_gear_sha256'],
                        'request_sha256': hashlib.sha256(data).hexdigest(), 'request': request})
    unchanged()
    result = {'mode': 'prepared_gear_window_requests', 'snapshot_id': summary['snapshot_id'],
        'scope_sha256': summary['scope_sha256'], 'source_report_sha256': report_sha,
        'window_offset_decimal': str(args.window_offset), 'resolution_accounting': summary['resolution_accounting'],
        'mcp_sha256': fingerprints[0], 'template_sha256': fingerprints[1], 'encounter_sha256': fingerprints[2],
        'preparer_sha256': fingerprints[3], 'adapter_sha256': fingerprints[4], 'catalogue_sha256': fingerprints[5],
        'layout_sha256':fingerprints[6],
        'talent_identity_sha256':fingerprints[7], 'talent_map_sha256':fingerprints[8],
        'captured_equipped_bag_items':bag_items,
        'requests': requests, 'simulation_performed': False, 'game_execution': False,
        'best_in_slot_verified': False, 'feasible_set_coverage_verified': False}
    require(len(encoded(result)) <= MAX_REPORT_BYTES)
    return result


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('--mcp', type=Path, required=True)
    cli.add_argument('--journal', type=Path, required=True)
    cli.add_argument('--window-offset', type=int, required=True)
    cli.add_argument('--template', type=Path, required=True)
    cli.add_argument('--encounter-lookup', type=Path, required=True)
    cli.add_argument('--time-budget-seconds', type=int, default=45)
    args = cli.parse_args(argv)
    async def run():
        integer(args.time_budget_seconds, 1, 45)
        return await asyncio.wait_for(prepare(args), args.time_budget_seconds)
    try:
        print(encoded(asyncio.run(run())).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Gear request preparation interrupted; journal unchanged.\n')
    except Exception:
        cli.exit(1, 'Gear request preparation rejected: invalid, changed, unresolved or unsupported local inputs.\n')


if __name__ == '__main__':
    main()
