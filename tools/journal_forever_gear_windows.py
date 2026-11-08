"""Persist owner-local plan windows transactionally; never execute saved commands."""
import argparse
import hashlib
import json
import os
from pathlib import Path
from math import isfinite
import sqlite3
import stat
from time import monotonic

from tools.merge_forever_gear_windows import ACCOUNTING_FIELDS, MAX_REPORT_BYTES, coordinate_outcomes, empty_accounting, integer, merge_reports, parse_report, require

APPLICATION_ID = 0x415A4757
SCHEMA = '''CREATE TABLE windows (
    start_key TEXT PRIMARY KEY,
    end_key TEXT NOT NULL,
    report BLOB NOT NULL CHECK(typeof(report)='blob'),
    report_sha TEXT NOT NULL,
    payload_bytes INTEGER NOT NULL CHECK(payload_bytes BETWEEN 1 AND 4194304)
)'''


def coordinate(value):
    integer(value)
    require(len(str(value)) <= 64)
    return str(value).zfill(64)


def encoded(report):
    data = json.dumps(report, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()
    require(len(data) <= MAX_REPORT_BYTES)
    return data


def connect(path, write=False, deadline=None):
    now = monotonic()
    if deadline is not None:
        require(type(deadline) in (int, float) and isfinite(deadline) and deadline > now)
    deadline = min(now + 45, deadline) if deadline is not None else now + 45
    path = Path(path)
    require(path.parent.resolve(strict=True).is_dir())
    try:
        info = path.lstat()
    except FileNotFoundError:
        require(write)
        try:
            os.close(os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600))
        except FileExistsError:
            pass
        info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and not path.is_symlink())
    if hasattr(os, 'geteuid'):
        require(info.st_uid == os.geteuid() and info.st_mode & 0o077 == 0)
    empty = info.st_size == 0
    if not empty:
        with path.open('rb') as handle:
            header = handle.read(100)
        require(len(header) == 100 and header[:16] == b'SQLite format 3\0'
                and int.from_bytes(header[68:72], 'big') == APPLICATION_ID
                and int.from_bytes(header[60:64], 'big') == 1)
    else:
        require(write)
    uri = path.resolve(strict=True).as_uri() + ('?mode=rw' if write else '?mode=ro')
    remaining = deadline - monotonic()
    require(remaining > 0)
    connection = sqlite3.connect(uri, uri=True, timeout=min(5, remaining), isolation_level=None)
    try:
        connection.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, MAX_REPORT_BYTES + 4096)
        connection.setlimit(sqlite3.SQLITE_LIMIT_SQL_LENGTH, 65536)
        connection.set_progress_handler(lambda: int(monotonic() > deadline), 1000)
        connection.execute('PRAGMA trusted_schema=OFF')
        connection.execute('BEGIN IMMEDIATE' if write else 'BEGIN')
        app_id = connection.execute('PRAGMA application_id').fetchone()[0]
        objects = connection.execute('SELECT type,name,sql FROM sqlite_master ORDER BY type,name LIMIT 3').fetchall()
        if empty and write and app_id == 0 and not objects:
            connection.execute(f'PRAGMA application_id={APPLICATION_ID}')
            connection.execute('PRAGMA user_version=1')
            connection.execute(SCHEMA)
            objects = connection.execute('SELECT type,name,sql FROM sqlite_master ORDER BY type,name LIMIT 3').fetchall()
        require(connection.execute('PRAGMA application_id').fetchone()[0] == APPLICATION_ID
                and connection.execute('PRAGMA user_version').fetchone()[0] == 1)
        require(len(objects) == 2 and objects[0] == ('index', 'sqlite_autoindex_windows_1', None)
                and objects[1][0:2] == ('table', 'windows')
                and ' '.join(objects[1][2].split()) == ' '.join(SCHEMA.split()))
        return connection, deadline
    except Exception:
        connection.close()
        raise


def checked_row(row):
    start, end, data, sha, size = row
    require(isinstance(data, bytes) and len(data) == integer(size, 1, MAX_REPORT_BYTES)
            and hashlib.sha256(data).hexdigest() == sha)
    report = parse_report(data)
    summary = merge_reports([report])
    window = report['gear_search']['combination_window']
    require(window['considered'] > 0 and start == coordinate(window['offset'])
            and end == coordinate(window['offset'] + window['considered']))
    return report, summary


def append_report(path, report, max_payload_bytes=64 * 1024 * 1024, deadline=None):
    integer(max_payload_bytes, 1, 512 * 1024 * 1024)
    summary = merge_reports([report])
    window = report['gear_search']['combination_window']
    require(window['considered'] > 0)
    start, end = coordinate(window['offset']), coordinate(window['offset'] + window['considered'])
    data = encoded(report)
    sha = hashlib.sha256(data).hexdigest()
    connection, deadline = connect(path, write=True, deadline=deadline)
    try:
        existing = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows WHERE start_key=?', (start,)).fetchone()
        if existing:
            checked_row(existing)
            require(existing[1] == end and existing[2] == data and existing[3] == sha)
            inserted = False
        else:
            anchor = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows ORDER BY start_key LIMIT 1').fetchone()
            if anchor:
                original, _ = checked_row(anchor)
                merge_reports([original, report])
            require(connection.execute('SELECT 1 FROM windows WHERE start_key < ? AND end_key > ? LIMIT 1',
                                       (end, start)).fetchone() is None)
            count, used = connection.execute('SELECT count(*),coalesce(sum(length(report)),0) FROM windows').fetchone()
            require(count < 1000000 and used + len(data) <= max_payload_bytes)
            connection.execute('INSERT INTO windows VALUES(?,?,?,?,?)', (start, end, data, sha, len(data)))
            inserted = True
        require(monotonic() <= deadline)
        connection.commit()
        return {'inserted': inserted, 'snapshot_id': summary['snapshot_id'], 'scope_sha256': summary['scope_sha256'],
                **({'collection_binding_sha256': summary['collection_binding_sha256']}
                   if 'collection_binding_sha256' in summary else {}),
                'offset_decimal': str(window['offset']), 'considered': window['considered'],
                'best_in_slot_verified': False, 'game_execution': False}
    finally:
        if connection.in_transaction:
            connection.rollback()
        connection.close()


def journal_status(path, deadline=None):
    connection, deadline = connect(path, deadline=deadline)
    try:
        anchor = summary = None
        cursor = covered = count = gap_count = 0
        gaps = []
        accounting = empty_accounting()
        content = hashlib.sha256()
        for row in connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows ORDER BY start_key'):
            require(monotonic() <= deadline)
            report, current = checked_row(row)
            content.update(encoded([row[0], row[1], row[3]]) + b'\n')
            for field in ACCOUNTING_FIELDS:
                accounting[field] += current['resolution_accounting'][field]
            if anchor is None:
                anchor, summary = report, current
            else:
                merge_reports([anchor, report])
            window = report['gear_search']['combination_window']
            start, end = window['offset'], window['offset'] + window['considered']
            require(start >= cursor)
            if start > cursor:
                gap_count += 1
                if len(gaps) < 64:
                    gaps.append({'start_decimal': str(cursor), 'end_decimal': str(start)})
            cursor = end
            covered += window['considered']
            count += 1
        total = int(summary['total_combinations_decimal']) if summary else None
        if total is not None and cursor < total:
            gap_count += 1
            if len(gaps) < 64:
                gaps.append({'start_decimal': str(cursor), 'end_decimal': str(total)})
        require(monotonic() <= deadline)
        return {'mode': 'gear_window_journal', 'snapshot_id': summary['snapshot_id'] if summary else None,
                **({'collection_binding_sha256': summary['collection_binding_sha256']}
                   if summary and 'collection_binding_sha256' in summary else {}),
                'scope_sha256': summary['scope_sha256'] if summary else None,
                'window_count': count, 'covered_coordinate_count': covered,
                'journal_content_sha256': content.hexdigest(),
                'equipment_specs_retained': summary['equipment_specs_retained'] if summary else None,
                'resolution_accounting': accounting,
                'total_combinations_decimal': str(total) if total is not None else None,
                'reported_coordinate_coverage_complete': total is not None and gap_count == 0,
                'uncovered_ranges': gaps, 'uncovered_range_count': gap_count, 'ranges_truncated': gap_count > len(gaps),
                'next_uncovered_offset_decimal': gaps[0]['start_decimal'] if gaps else ('0' if total is None else None),
                'feasible_set_coverage_verified': False, 'best_in_slot_verified': False,
                'simulation_performed': False, 'game_execution': False}
    finally:
        connection.close()


def journal_window(path, offset, include_equipment_specs=False):
    require(type(include_equipment_specs) is bool)
    start = coordinate(offset)
    connection, deadline = connect(path)
    try:
        row = connection.execute('SELECT start_key,end_key,report,report_sha,payload_bytes FROM windows WHERE start_key=?', (start,)).fetchone()
        require(row is not None)
        report, summary = checked_row(row)
        outcomes = coordinate_outcomes(report)
        require(monotonic() <= deadline)
        return {'mode': 'gear_window_inspection', 'snapshot_id': summary['snapshot_id'],
                'scope_sha256': summary['scope_sha256'], 'offset_decimal': str(offset),
                'considered': report['gear_search']['combination_window']['considered'],
                'coordinate_outcomes_available': outcomes is not None, 'coordinate_outcomes': outcomes,
                'resolution_accounting': summary['resolution_accounting'],
                **({'equipment_specs_available': summary['equipment_specs_retained'],
                    'equipment_specs': [{'review': row['review'],
                        'simulator_gear_sha256': row['simulator_gear_sha256'],
                        'simulator_equipment_spec': row['simulator_equipment_spec']}
                        for row in report['resolved_drafts']] if summary['equipment_specs_retained'] else None}
                   if include_equipment_specs else {}),
                'best_in_slot_verified': False, 'feasible_set_coverage_verified': False,
                'simulation_performed': False, 'game_execution': False}
    finally:
        connection.close()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--journal', required=True, type=Path)
    action = parser.add_mutually_exclusive_group()
    action.add_argument('--add', type=Path, help='Append one validated plan report; omission reads progress only.')
    action.add_argument('--window-offset', type=int, help='Inspect coordinate outcomes at an exact saved window start.')
    parser.add_argument('--include-equipment-specs', action='store_true',
                        help='Include retained numeric inputs when inspecting an exact window')
    parser.add_argument('--max-payload-mib', type=int, default=64, choices=range(1, 513), metavar='1-512')
    args = parser.parse_args(argv)
    try:
        require(not args.include_equipment_specs or args.window_offset is not None)
        if args.add is not None:
            with args.add.open('rb') as handle:
                data = handle.read(MAX_REPORT_BYTES + 1)
            result = append_report(args.journal, parse_report(data), args.max_payload_mib * 1024 * 1024)
        elif args.window_offset is not None:
            result = journal_window(args.journal, args.window_offset, args.include_equipment_specs)
        else:
            result = journal_status(args.journal)
        print(json.dumps(result, allow_nan=False))
    except Exception:
        parser.exit(1, 'Gear window journal rejected: invalid, incompatible, busy or over-budget local data.\n')


if __name__ == '__main__':
    main()
