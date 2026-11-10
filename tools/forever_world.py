"""Sync pinned public QuestieDB sources, build an offline cache and serve exact Forever world records."""
import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from typing import Annotated, Literal

from tools.sync_exports import ordinary

ROOT = Path(__file__).resolve().parents[1]
PIN = json.loads((ROOT/'clients/forever/world.lock.json').read_text())
MAX_BYTES = 64 * 1024 * 1024
MAX_RESPONSE = 256 * 1024
Kind = Literal['quest', 'npc', 'object', 'item']
KINDS = ('quest', 'npc', 'object', 'item')
SOURCE_PATTERNS = ('/src/', '/generator/', '/data/Forever/', '/tools/export/',
                   '/tools/lua-binary/lua.exe', '/LICENSE.md', '/gpl-3.0.md', '/PROVENANCE.md')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def bounded_read(path, limit):
    path = ordinary(path.absolute())
    if not path.is_file():
        raise ValueError('World input must be an ordinary file')
    with path.open('rb') as handle:
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise ValueError('World input exceeds byte limit')
    return data


def verify_source(source, destination=None):
    total = 0
    for name, expected in PIN['source_files'].items():
        # These are trusted repository lock entries, never caller-supplied paths.
        parts = name.split('/')
        if any(part in ('', '.', '..') or ':' in part or '\\' in part for part in parts):
            raise ValueError('Invalid world source pin')
        data = bounded_read(source.joinpath(*parts), MAX_BYTES - total)
        total += len(data)
        if digest(data) != expected:
            raise ValueError('World source checksum mismatch')
        if destination is not None:
            staged = destination.joinpath(*parts)
            staged.parent.mkdir(parents=True, exist_ok=True)
            staged.write_bytes(data)
    return total


def sync_source(destination):
    """Explicit network action; never invoked by build, get, search or serve."""
    destination = ordinary(destination.absolute())
    if destination.exists():
        return {'status': 'unchanged', 'source_bytes': verify_source(destination), **provenance()}
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.world-source-', dir=destination.parent) as folder:
        stage = Path(folder)
        checkout = stage/'checkout'
        env = {**os.environ, 'GIT_TERMINAL_PROMPT': '0', 'GIT_CONFIG_NOSYSTEM': '1',
               'GIT_CONFIG_GLOBAL': os.devnull}
        # Do not inherit caller Git directory overrides or command-line config.
        env = {key: value for key, value in env.items() if not key.startswith('GIT_') or key in
               ('GIT_TERMINAL_PROMPT', 'GIT_CONFIG_NOSYSTEM', 'GIT_CONFIG_GLOBAL')}
        def git(*args):
            subprocess.run(['git', '-c', 'credential.helper=', '-c', 'core.hooksPath=.git/no-hooks', *args],
                cwd=stage, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, timeout=60, check=True)
        git('init', str(checkout))
        git('-C', str(checkout), 'config', 'core.autocrlf', 'false')
        git('-C', str(checkout), 'remote', 'add', 'origin', f"https://github.com/{PIN['provider']}.git")
        git('-C', str(checkout), 'sparse-checkout', 'set', '--no-cone', *SOURCE_PATTERNS)
        git('-C', str(checkout), '-c', 'protocol.version=2', 'fetch', '--depth=1', '--filter=blob:none',
            'origin', PIN['revision'])
        git('-C', str(checkout), 'checkout', '--detach', PIN['revision'])
        verified = stage/'verified'
        total = verify_source(checkout, verified)
        # On the supported Windows host rename refuses an existing destination.
        if destination.exists():
            raise ValueError('World source destination changed')
        verified.rename(destination)
    return {'status': 'synced', 'source_bytes': total, **provenance()}


def provenance():
    return {'provider': PIN['provider'], 'source_revision': PIN['revision'],
            'cache_sha256': PIN['cache_sha256'], 'scope': 'pinned_public_forever_generation',
            'dynamic_character_corrections_applied': False, 'localization_applied': False,
            'complete_world_data_verified': False, 'native_availability_verified': False,
            'game_execution': False, 'live': False}


def check_cache(path):
    data = bounded_read(path, MAX_BYTES)
    if digest(data) != PIN['cache_sha256']:
        raise ValueError('World cache checksum mismatch')
    return data


def build(source, cache):
    cache = ordinary(cache.absolute())
    if cache.exists():
        check_cache(cache)
        return {'status': 'unchanged', **provenance()}
    cache.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.world-build-', dir=cache.parent) as folder:
        stage = Path(folder)
        verify_source(source, stage)
        allow = stage/'allowed.txt'
        allow.write_text('\n'.join(sorted(PIN['source_files']))+'\n', encoding='ascii')
        output = stage/'world.jsonl'
        # No inherited LUA_INIT, PATH, user credentials or provider installation paths.
        env = {key: os.environ[key] for key in ('SystemRoot',) if key in os.environ}
        with output.open('wb') as handle:
            subprocess.run([str(stage/'tools/lua-binary/lua.exe'), str(ROOT/'tools/export_forever_world.lua'), str(allow)],
                cwd=stage, env=env, stdin=subprocess.DEVNULL, stdout=handle, stderr=subprocess.DEVNULL,
                timeout=45, check=True)
            handle.flush()
            os.fsync(handle.fileno())
        check_cache(output)
        # Same-volume atomic publication without overwrite, even if a caller races us.
        os.link(output, cache)
    return {'status': 'built', **provenance()}


def encoded(value):
    data = json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)
    if len(data.encode()) > MAX_RESPONSE:
        raise ValueError('World response exceeds byte limit')
    return data


class World:
    def __init__(self, cache):
        lines = check_cache(cache).splitlines()
        self.header = json.loads(lines[0])
        self.records = {}
        counts = Counter()
        for line in lines[1:]:
            row = json.loads(line)
            kind, identity, name = row['kind'], row['id'], row['fields']['name']
            key = (kind, identity)
            if (kind not in KINDS or type(identity) is not int or not 1 <= identity <= 2**32-1
                    or type(name) is not str or key in self.records):
                raise ValueError('Invalid pinned world record')
            self.records[key] = (row, digest(line), ' '.join(name.casefold().split()))
            counts[kind] += 1
        if dict(counts) != PIN['counts'] or self.header != PIN['generation']:
            raise ValueError('Invalid pinned world coverage')

    def metadata(self):
        return {**provenance(), 'generation': dict(self.header), 'record_counts': dict(PIN['counts'])}

    def summary(self, key):
        row, sha, _ = self.records[key]
        return {'kind': key[0], 'id': key[1], 'record_key': f'questiedb:{key[0]}:{key[1]}',
                'name': row['fields']['name'], 'record_sha256': sha}

    def get(self, kind, entity_id, *, include_quest_links=False):
        if kind not in KINDS or type(entity_id) is not int or not 1 <= entity_id <= 2**32-1:
            raise ValueError('Invalid world record key')
        key = (kind, entity_id)
        row = None if key not in self.records else {
            **self.summary(key), 'fields': deepcopy(self.records[key][0]['fields'])}
        result = {**self.metadata(), 'found': row is not None, 'record': row}
        if include_quest_links:
            if kind != 'quest':
                raise ValueError('Quest links require a quest record')
            result['quest_links'] = None if row is None else self.quest_links(row['fields'])
        encoded(result)
        return result

    def quest_links(self, fields):
        links = {}
        for field in ('preQuestGroup', 'preQuestSingle', 'childQuests', 'inGroupWith', 'exclusiveTo'):
            entries = fields.get(field)
            if not isinstance(entries, dict):
                continue
            links[field] = {}
            for position, identity in entries.items():
                if type(identity) is not int or not 1 <= identity <= 2**32-1:
                    links[field][position] = {'status': 'unsupported_reference'}
                    continue
                key = ('quest', identity)
                links[field][position] = {
                    'quest_id': identity,
                    'record': self.summary(key) if key in self.records else None,
                }
        return links

    def search(self, query, kind=None, limit=10, offset=0):
        if (type(query) is not str or len(query.encode()) > 128 or not query.strip()
                or kind is not None and kind not in KINDS or type(limit) is not int or not 1 <= limit <= 20
                or type(offset) is not int or not 0 <= offset <= 2**32-1):
            raise ValueError('Invalid world name query')
        words = query.casefold().split()
        phrase = ' '.join(words)
        hits = []
        for key, (_, _, name) in self.records.items():
            if (kind is None or key[0] == kind) and all(word in name for word in words):
                hits.append((0 if name == phrase else 1 if name.startswith(phrase) else 2, name, key))
        hits.sort()
        result = {**self.metadata(), 'match_count': len(hits), 'offset': offset, 'limit': limit,
            'has_more': offset+limit < len(hits), 'results': [self.summary(key) for _, _, key in hits[offset:offset+limit]]}
        encoded(result)
        return result


def serve(world):
    from mcp.server.fastmcp import FastMCP
    from mcp.types import ToolAnnotations
    from pydantic import Field, StrictBool, StrictInt
    server = FastMCP('Azeroth Forever World References')
    readonly = ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False)

    @server.tool(annotations=readonly)
    def get_forever_world_record(kind: Kind, entity_id: Annotated[StrictInt, Field(ge=1, le=4294967295)],
                                 include_quest_links: StrictBool = False) -> dict:
        """Read one exact quest, NPC, object or item from the pinned local QuestieDB Forever export. Static corrections and derived data only; no character-specific eligibility, routing or current availability."""
        return world.get(kind, entity_id, include_quest_links=include_quest_links)

    @server.tool(annotations=readonly)
    def search_forever_world_records(query: str, kind: Kind | None = None,
            limit: Annotated[StrictInt, Field(ge=1, le=20)] = 10,
            offset: Annotated[StrictInt, Field(ge=0, le=4294967295)] = 0) -> dict:
        """Search local Forever entity names by literal case-insensitive terms, with stable pages and exact record keys. Max query 128 UTF-8 bytes. No external lookup or character data."""
        try:
            return world.search(query, kind, limit, offset)
        except ValueError:
            raise ValueError('Invalid or oversized world name query') from None
    server.run(transport='stdio')


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('action', choices=('sync-source','build','get','search','serve'))
    cli.add_argument('--source', type=Path)
    cli.add_argument('--cache', type=Path)
    cli.add_argument('--kind', choices=KINDS)
    cli.add_argument('--entity-id', type=int)
    cli.add_argument('--query')
    cli.add_argument('--limit', type=int, default=10)
    cli.add_argument('--offset', type=int, default=0)
    args = cli.parse_args(argv)
    try:
        if args.action == 'sync-source' and args.source is not None:
            result = sync_source(args.source)
        elif args.action == 'build' and args.source is not None and args.cache is not None:
            result = build(args.source, args.cache)
        elif args.action in ('get','search','serve') and args.cache is not None:
            world = World(args.cache)
            if args.action == 'serve':
                serve(world)
                return
            result = world.get(args.kind, args.entity_id) if args.action == 'get' else world.search(
                args.query, args.kind, args.limit, args.offset)
        else:
            raise ValueError('Required explicit paths unavailable')
        print(encoded(result))
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        cli.exit(1, 'Forever world operation stopped: invalid, unavailable or over-budget pinned inputs.\n')


if __name__ == '__main__':
    main()
