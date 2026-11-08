"""Private, non-evicting numeric simulation checkpoints for the fixed-run adapter."""
from contextlib import contextmanager
import hashlib
import json
import math
import os
from pathlib import Path
import sqlite3
import stat
from time import monotonic

if __package__:
    from .simulation_cache import SimulationCache, encoded, private_windows_path, user_protected
    from .sync_exports import ordinary
else:
    from simulation_cache import SimulationCache, encoded, private_windows_path, user_protected
    from sync_exports import ordinary


def require(condition):
    if not condition: raise ValueError('Invalid or incompatible simulation checkpoint')


def integer(value, minimum, maximum):
    require(type(value) is int and minimum <= value <= maximum)
    return value


def digest(value):
    require(isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value))
    return value


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result

APPLICATION_ID = 0x415A5352
MAX_ENTRY_BYTES = 512 * 1024
SCHEMA = '''CREATE TABLE results (
    key TEXT PRIMARY KEY,
    body BLOB NOT NULL CHECK(typeof(body)='blob'),
    digest TEXT NOT NULL,
    plain_bytes INTEGER NOT NULL CHECK(plain_bytes BETWEEN 1 AND 524288)
)'''


class SimulationJournal:
    def __init__(self, root, max_payload_bytes=64 * 1024 * 1024, max_entries=1000000):
        self.root = Path(root)
        require(self.root.is_absolute())
        self.max_payload_bytes = integer(max_payload_bytes, 1, 512 * 1024 * 1024)
        self.max_entries = integer(max_entries, 1, 1000000)
        self.bindings = {}

    def key(self, engine_hash, request, paired, tank_metrics, healing_metrics):
        require(isinstance(engine_hash, str))
        engine_hash = digest(engine_hash.lower())
        require(all(type(flag) is bool for flag in [paired, tank_metrics, healing_metrics]))
        require(isinstance(request, dict) and len(encoded(request)) <= 4 * 1024 * 1024)
        options = request['simOptions']
        iterations = integer(options['iterations'], 2 if paired else 1, 5000 if paired else 100000)
        seed = options['randomSeed']
        require(isinstance(seed, str) and seed.isascii() and seed.isdecimal()
                and 1 <= int(seed) <= 2**32 - 1)
        fixed_key = SimulationCache(None).key(engine_hash, request, paired, tank_metrics, healing_metrics)
        sources = {}
        for path in [Path(__file__), Path(__file__).with_name('run_forever_sim.py')]:
            with path.open('rb') as handle: sources[path.name] = hashlib.file_digest(handle, 'sha256').hexdigest()
        key = hashlib.sha256(encoded({'journal_version': 1, 'fixed_run_key': fixed_key, 'sources': sources})).hexdigest()
        require(key in self.bindings or len(self.bindings) < 1000000)
        self.bindings[key] = (engine_hash, iterations, seed, paired, tank_metrics, healing_metrics)
        return key

    def validate(self, key, summary):
        digest(key)
        require(key in self.bindings and isinstance(summary, dict))
        engine, iterations, seed, paired, tank, healing = self.bindings[key]
        required = {'engine_sha256', 'iterations', 'seed', 'mean_dps', 'dps_standard_deviation'}
        if tank:
            required |= {'mean_dtps', 'dtps_standard_deviation', 'mean_tps', 'tps_standard_deviation',
                'simulated_death_fraction', 'simulated_deaths', 'simulated_death_fraction_wilson_95'}
        if healing: required |= {'mean_hps', 'hps_standard_deviation', 'mean_seconds_oom'}
        samples = ['dps'] + (['dtps', 'tps'] if tank else []) + (['hps'] if healing else [])
        if paired: required |= {f'_{metric}_samples' for metric in samples}
        optional = {'mean_iteration_duration_seconds', 'simulated_time_oom_fraction'} if healing else set()
        require(required <= set(summary) <= required | optional)
        require(summary['engine_sha256'] == engine and summary['seed'] == seed
                and integer(summary['iterations'], 1, 100000) == iterations)
        def finite(value):
            require(type(value) in (int, float) and math.isfinite(value))
        for field, value in summary.items():
            if field in {'engine_sha256', 'iterations', 'seed', 'simulated_death_fraction_wilson_95'} or field.startswith('_'):
                continue
            finite(value)
            if field != 'mean_tps': require(value >= 0)
        if tank:
            integer(summary['simulated_deaths'], 0, iterations)
            require(0 <= summary['simulated_death_fraction'] <= 1)
            interval = summary['simulated_death_fraction_wilson_95']
            require(isinstance(interval, dict) and set(interval) == {'lower', 'upper'})
            for value in interval.values(): finite(value)
            require(0 <= interval['lower'] <= interval['upper'] <= 1)
        if 'mean_iteration_duration_seconds' in summary: require(summary['mean_iteration_duration_seconds'] > 0)
        if 'simulated_time_oom_fraction' in summary:
            require('mean_iteration_duration_seconds' in summary and 0 <= summary['simulated_time_oom_fraction'] <= 1)
        if paired:
            for metric in samples:
                values = summary[f'_{metric}_samples']
                require(isinstance(values, list) and len(values) == iterations)
                for value in values: finite(value)
        data = encoded(summary)
        require(1 <= len(data) <= MAX_ENTRY_BYTES)
        return data

    @contextmanager
    def connection(self):
        deadline = monotonic() + 45
        ordinary(self.root).mkdir(parents=True, exist_ok=True, mode=0o700)
        def private(path):
            ordinary(path)
            info = path.stat()
            require((private_windows_path(path) if os.name == 'nt' else
                     info.st_uid == os.getuid() and info.st_mode & 0o077 == 0))
        private(self.root)
        path = ordinary(self.root / 'checkpoints.sqlite3')
        if not path.exists():
            try: os.close(os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600))
            except FileExistsError: pass
        private(path)
        require(stat.S_ISREG(path.stat().st_mode))
        for suffix in ['-journal', '-wal', '-shm']:
            sidecar = ordinary(Path(str(path) + suffix))
            if sidecar.exists(): private(sidecar)
        empty = path.stat().st_size == 0
        if not empty:
            with path.open('rb') as handle: header = handle.read(100)
            require(len(header) == 100 and header[:16] == b'SQLite format 3\0'
                    and int.from_bytes(header[68:72], 'big') == APPLICATION_ID
                    and int.from_bytes(header[60:64], 'big') == 1)
        connection = sqlite3.connect(path.as_uri() + '?mode=rw', uri=True, timeout=5, isolation_level=None)
        try:
            connection.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, MAX_ENTRY_BYTES + 20480)
            connection.setlimit(sqlite3.SQLITE_LIMIT_SQL_LENGTH, 65536)
            connection.set_progress_handler(lambda: int(monotonic() > deadline), 1000)
            connection.execute('PRAGMA trusted_schema=OFF')
            connection.execute('BEGIN IMMEDIATE')
            objects = connection.execute('SELECT type,name,sql FROM sqlite_master ORDER BY type,name LIMIT 3').fetchall()
            if empty and not objects and connection.execute('PRAGMA application_id').fetchone()[0] == 0:
                connection.execute(f'PRAGMA application_id={APPLICATION_ID}')
                connection.execute('PRAGMA user_version=1')
                connection.execute(SCHEMA)
                objects = connection.execute('SELECT type,name,sql FROM sqlite_master ORDER BY type,name LIMIT 3').fetchall()
            require(connection.execute('PRAGMA application_id').fetchone()[0] == APPLICATION_ID
                    and connection.execute('PRAGMA user_version').fetchone()[0] == 1)
            require(len(objects) == 2 and objects[0] == ('index', 'sqlite_autoindex_results_1', None)
                    and objects[1][:2] == ('table', 'results')
                    and ' '.join(objects[1][2].split()) == ' '.join(SCHEMA.split()))
            yield connection, deadline
        finally:
            if connection.in_transaction: connection.rollback()
            connection.close()

    def read_row(self, key, row):
        body, sha, size = row
        require(isinstance(body, bytes) and len(body) <= MAX_ENTRY_BYTES + 16384
                and hashlib.sha256(body).hexdigest() == sha)
        data = user_protected(body, key, decrypt=True)
        require(len(data) == integer(size, 1, MAX_ENTRY_BYTES))
        payload = json.loads(data, object_pairs_hook=unique_fields)
        require(isinstance(payload, dict) and set(payload) == {'key', 'summary'} and payload['key'] == key)
        summary = payload['summary']
        self.validate(key, summary)
        require(encoded(payload) == data)
        return summary

    def get(self, key):
        digest(key)
        require(key in self.bindings)
        with self.connection() as (connection, deadline):
            row = connection.execute('SELECT body,digest,plain_bytes FROM results WHERE key=?', (key,)).fetchone()
            summary = self.read_row(key, row) if row is not None else None
            require(monotonic() <= deadline)
            connection.commit()
            return summary

    def put(self, key, summary):
        summary_data = self.validate(key, summary)
        data = encoded({'key': key, 'summary': summary})
        require(len(data) <= MAX_ENTRY_BYTES)
        body = user_protected(data, key)
        require(len(body) <= MAX_ENTRY_BYTES + 16384)
        with self.connection() as (connection, deadline):
            row = connection.execute('SELECT body,digest,plain_bytes FROM results WHERE key=?', (key,)).fetchone()
            if row is not None:
                require(encoded(self.read_row(key, row)) == summary_data)
            else:
                count, used = connection.execute('SELECT count(*),coalesce(sum(length(body)),0) FROM results').fetchone()
                require(count < self.max_entries and used + len(body) <= self.max_payload_bytes)
                connection.execute('INSERT INTO results VALUES(?,?,?,?)',
                    (key, body, hashlib.sha256(body).hexdigest(), len(data)))
            require(monotonic() <= deadline)
            connection.commit()
