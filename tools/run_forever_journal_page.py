"""Run the next bounded page of saved gear windows with resumable checkpoints."""
import argparse
import asyncio
from copy import copy
import hashlib
from pathlib import Path
from math import isfinite
from time import monotonic

from tools import journal_forever_gear_windows as journal
from tools import run_forever_encounter_windows as encounters
from tools.merge_forever_gear_windows import MAX_TOTAL_BYTES, digest, integer, merge_reports, require
from tools.prepare_forever_gear_window import encoded
from tools import role_metrics as roles


def select_page(path, after, limit, deadline):
    integer(limit, 1, 64)
    if after is not None: journal.coordinate(after)
    connection, deadline = journal.connect(path, deadline=deadline)
    try:
        rows = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows '
            'WHERE start_key > ? ORDER BY start_key LIMIT ?',
            (journal.coordinate(after) if after is not None else '', limit))
        reports, identities, offsets, size = [], [], [], 0
        for row in rows:
            require(monotonic() <= deadline)
            size += len(row[2])
            require(size <= MAX_TOTAL_BYTES)
            report, summary = journal.checked_row(row)
            require(summary['equipment_specs_retained'])
            reports.append(report)
            offsets.append(report['gear_search']['combination_window']['offset'])
            identities.append([row[0], row[1], row[3]])
        if reports:
            merge_reports(reports)
            next_row = connection.execute('SELECT start_key FROM windows WHERE start_key > ? '
                'ORDER BY start_key LIMIT 1', (identities[-1][0],)).fetchone()
            if next_row is not None:
                require(isinstance(next_row[0], str) and len(next_row[0]) == 64
                    and next_row[0].isascii() and next_row[0].isdecimal()
                    and journal.coordinate(int(next_row[0])) == next_row[0])
            next_offset = str(int(next_row[0])) if next_row else None
        else:
            next_offset = None
        require(monotonic() <= deadline)
        return {'after_window_offset_decimal': str(after) if after is not None else None,
            'selected_window_offsets_decimal': [str(offset) for offset in offsets],
            'selected_window_count': len(offsets), 'page_limit': limit,
            'selection_sha256': hashlib.sha256(encoded(identities)).hexdigest(),
            'next_saved_window_offset_decimal': next_offset}
    finally:
        connection.close()


async def execute(args, *, deadline=None):
    metric = roles.rank_by(args)
    integer(args.time_budget_seconds, 1, 3600)
    integer(args.max_new_runs, 1, 128)
    integer(args.iteration_budget, 1, 100000)
    integer(args.simulation_journal_mib, 1, 512)
    require(1 <= len(args.encounter_lookup) <= 8)
    encounters.normalise_weights(args.encounter_weight, len(args.encounter_lookup))
    digest(args.engine_sha256.lower())
    if deadline is not None:
        require(type(deadline) in (int, float) and isfinite(deadline) and deadline > monotonic())
    deadline = min(monotonic() + args.time_budget_seconds, deadline) if deadline is not None else monotonic() + args.time_budget_seconds
    selected = select_page(args.journal, args.after_window_offset, args.page_limit, deadline)
    offsets = [int(value) for value in selected['selected_window_offsets_decimal']]
    result = None
    if offsets:
        options = copy(args)
        options.window_offset = offsets
        remaining = deadline - monotonic()
        require(remaining > 0)
        result = await asyncio.wait_for(encounters.execute(options, deadline=deadline), remaining + 5)
    current = select_page(args.journal, args.after_window_offset, args.page_limit, deadline + 5)
    require(current == selected and monotonic() <= deadline + 5)
    complete = result is not None and result['requested_results_complete'] is True
    selected['page_results_complete'] = complete
    selected['resume_after_window_offset_decimal'] = (
        str(offsets[-1]) if complete and selected['next_saved_window_offset_decimal'] is not None
        else None if complete or not offsets
        else selected['after_window_offset_decimal'])
    selected['resume_same_page'] = bool(offsets) and not complete
    selected['prior_pages_verified'] = False
    report = {'mode': 'saved_journal_page_execution', 'journal_page': selected,
        'rank_by':metric,
        'page_status': 'no_saved_windows_after_cursor' if not offsets else 'complete' if complete else 'incomplete',
        'encounter_comparison': result, 'simulation_performed': result['simulation_performed'] if result else False,
        'global_ranking_verified': False, 'feasible_set_coverage_verified': False,
        'best_in_slot_verified': False, 'game_execution': False}
    require(len(encoded(report)) <= MAX_TOTAL_BYTES and monotonic() <= deadline + 5)
    return report


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ['mcp', 'journal', 'template', 'engine', 'simulation-journal']:
        cli.add_argument('--' + name, type=Path, required=True)
    cli.add_argument('--encounter-lookup', type=Path, action='append', required=True)
    cli.add_argument('--encounter-weight', type=float, action='append')
    cli.add_argument('--engine-sha256', required=True)
    cli.add_argument('--after-window-offset', type=int)
    cli.add_argument('--page-limit', type=int, default=8)
    cli.add_argument('--paired', action='store_true')
    cli.add_argument('--rank-by',choices=roles.FIELDS,default='dps')
    cli.add_argument('--checkpoint-only', action='store_true')
    cli.add_argument('--max-new-runs', type=int, default=16)
    cli.add_argument('--iteration-budget', type=int, default=100000)
    cli.add_argument('--time-budget-seconds', type=int, default=180)
    cli.add_argument('--simulation-journal-mib', type=int, default=64)
    args = cli.parse_args(argv)
    try:
        print(encoded(asyncio.run(execute(args))).decode())
    except KeyboardInterrupt:
        cli.exit(130, 'Journal page interrupted; completed numeric checkpoints remain available.\n')
    except Exception:
        cli.exit(1, 'Journal page stopped: invalid, changed, unsupported or over-budget local inputs; numeric checkpoints remain available.\n')


if __name__ == '__main__':
    main()
