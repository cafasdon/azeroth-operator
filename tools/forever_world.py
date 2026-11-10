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

    def get(self, kind, entity_id, *, include_quest_links=False, include_item_sources=False,
            item_source_offset=0):
        if kind not in KINDS or type(entity_id) is not int or not 1 <= entity_id <= 2**32-1:
            raise ValueError('Invalid world record key')
        key = (kind, entity_id)
        row = None if key not in self.records else {
            **self.summary(key), 'fields': deepcopy(self.records[key][0]['fields'])}
        result = {**self.metadata(), 'found': row is not None, 'record': row}
        if include_quest_links:
            fields = None if row is None else row['fields']
            result['quest_links'] = None if fields is None else self.quest_links(
                fields, kind=kind)
            if kind == 'quest':
                result['quest_contacts'] = None if fields is None else self.quest_contacts(fields)
                result['quest_objective_targets'] = None if fields is None else self.quest_objective_targets(fields)
        if (type(item_source_offset) is not int or not 0 <= item_source_offset <= 2**32-1
                or item_source_offset and not include_item_sources):
            raise ValueError('Invalid item source offset')
        if include_item_sources:
            if kind == 'item':
                result['item_sources'] = None if row is None else self.item_sources(row['fields'], item_source_offset)
            result['sourced_items'] = None if row is None else self.sourced_items(kind, entity_id, item_source_offset)
        encoded(result)
        return result

    def sourced_items(self, kind, entity_id, offset):
        fields = {
            'npc': ('npcDrops', 'vendors'),
            'object': ('objectDrops',),
            'quest': ('questRewards',),
            'item': ('itemDrops',),
        }[kind]
        matches = {field: [] for field in fields}
        for key, (row, _, _) in self.records.items():
            if key[0] != 'item':
                continue
            for field in fields:
                entries = row['fields'].get(field)
                if isinstance(entries, dict) and any(
                    type(value) is int and value == entity_id for value in entries.values()
                ):
                    matches[field].append(key)
        return {
            field: {'count': len(keys), 'offset': offset, 'has_more': offset + 32 < len(keys),
                    'items': [self.summary(key) for key in sorted(keys)[offset:offset+32]]}
            for field, keys in matches.items()
        }

    def item_sources(self, fields, offset):
        sources = {}
        for field, kind in (
            ('npcDrops', 'npc'), ('objectDrops', 'object'), ('itemDrops', 'item'),
            ('questRewards', 'quest'), ('vendors', 'npc'),
        ):
            entries = fields.get(field)
            if not isinstance(entries, dict):
                continue
            references = {}
            page = sorted(entries.items(), key=lambda entry: int(entry[0]))[offset:offset+32]
            for position, identity in page:
                if type(identity) is not int or not 1 <= identity <= 2**32-1:
                    references[position] = {'status': 'unsupported_reference'}
                    continue
                key = (kind, identity)
                references[position] = {
                    'entity_id': identity,
                    'record': self.summary(key) if key in self.records else None,
                }
            sources[field] = {'kind': kind, 'references': references, 'count': len(entries),
                              'offset': offset, 'has_more': offset+32 < len(entries)}
        return sources

    def quest_objective_targets(self, fields):
        objectives = fields.get('objectives', {})
        targets = {}
        groups = [(position, kind, objectives.get(position), True)
                  for position, kind in (('1', 'npc'), ('2', 'object'), ('3', 'item'))]
        if 'sourceItemId' in fields:
            groups.append(('sourceItemId', 'item', {'1': fields['sourceItemId']}, False))
        groups.append(('requiredSourceItems', 'item', fields.get('requiredSourceItems'), False))
        for position, kind, group, tuple_entries in groups:
            if not isinstance(group, dict):
                continue
            references = {}
            for slot, objective in group.items():
                identity = objective
                if tuple_entries:
                    identity = objective.get('1') if isinstance(objective, dict) else None
                if type(identity) is not int or not 1 <= identity <= 2**32-1:
                    references[slot] = {'status': 'unsupported_reference'}
                    continue
                key = (kind, identity)
                references[slot] = {
                    'entity_id': identity,
                    'record': self.summary(key) if key in self.records else None,
                }
            targets[position] = {'kind': kind, 'references': references}
        return targets

    def quest_links(self, fields, *, kind='quest'):
        links = {}
        scalar_fields = {
            'item': ('startQuest',),
            'quest': ('nextQuestInChain', 'parentQuest', 'breadcrumbForQuestId',
                      'availableUntilCompleted', 'availableStartingWith', 'disabledByQuest'),
        }.get(kind, ())
        if kind == 'item':
            relationship_fields = ('startQuest', 'relatedQuests')
        elif kind == 'quest':
            relationship_fields = ('preQuestGroup', 'preQuestSingle', 'childQuests',
                                   'inGroupWith', 'exclusiveTo', 'breadcrumbs') + scalar_fields
        else:
            relationship_fields = ('questStarts', 'questEnds')
        for field in relationship_fields:
            entries = fields.get(field)
            if field in scalar_fields and field in fields:
                entries = {'1': entries}
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

    def quest_contacts(self, fields):
        contacts = {}
        kinds = {'1': 'npc', '2': 'object', '3': 'item'}
        for field in ('startedBy', 'finishedBy'):
            groups = fields.get(field)
            if not isinstance(groups, dict):
                continue
            contacts[field] = {}
            for position, entries in groups.items():
                kind = kinds.get(position)
                if kind is None or not isinstance(entries, dict):
                    contacts[field][position] = {'status': 'unsupported_reference'}
                    continue
                references = {}
                for slot, identity in entries.items():
                    if type(identity) is not int or not 1 <= identity <= 2**32-1:
                        references[slot] = {'status': 'unsupported_reference'}
                        continue
                    key = (kind, identity)
                    references[slot] = {
                        'entity_id': identity,
                        'record': self.summary(key) if key in self.records else None,
                    }
                contacts[field][position] = {'kind': kind, 'references': references}
        return contacts

    def search(self, query, kind=None, limit=10, offset=0, *, spawn_area_id=None):
        if (spawn_area_id is not None and
                (type(spawn_area_id) is not int or not 1 <= spawn_area_id <= 2**32-1
                 or kind not in (None, 'npc', 'object'))):
            raise ValueError('Invalid spawn area filter')
        if (type(query) is not str or len(query.encode()) > 128
                or not query.strip() and spawn_area_id is None
                or kind is not None and kind not in KINDS or type(limit) is not int or not 1 <= limit <= 20
                or type(offset) is not int or not 0 <= offset <= 2**32-1):
            raise ValueError('Invalid world name query')
        words = query.casefold().split()
        phrase = ' '.join(words)
        area_key = None if spawn_area_id is None else str(spawn_area_id)
        hits = []
        for key, (row, _, name) in self.records.items():
            if area_key is not None:
                spawns = row['fields'].get('spawns')
                points = spawns.get(area_key) if isinstance(spawns, dict) else None
                if key[0] not in ('npc', 'object') or not isinstance(points, dict) or not points:
                    continue
            if (kind is None or key[0] == kind) and all(word in name for word in words):
                hits.append((0 if name == phrase else 1 if name.startswith(phrase) else 2, name, key))
        hits.sort()
        result = {**self.metadata(), 'match_count': len(hits), 'offset': offset, 'limit': limit,
            'has_more': offset+limit < len(hits), 'results': [self.summary(key) for _, _, key in hits[offset:offset+limit]]}
        if spawn_area_id is not None:
            result['spawn_area_id'] = spawn_area_id
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
                                 include_quest_links: StrictBool = False,
                                 include_item_sources: StrictBool = False,
                                 item_source_offset: Annotated[StrictInt, Field(ge=0, le=4294967295)] = 0) -> dict:
        """Read an exact record from the pinned local QuestieDB Forever export.

        Enable include_quest_links for related quests and for quest contacts and objective targets.
        Enable include_item_sources for acquisition sources, container contents, quest rewards,
        or items from NPCs and objects. Advance item_source_offset by 32 to page those results.
        Records describe static world data. Live availability and character eligibility are unknown.
        """
        return world.get(kind, entity_id, include_quest_links=include_quest_links,
                         include_item_sources=include_item_sources, item_source_offset=item_source_offset)

    @server.tool(annotations=readonly)
    def search_forever_world_records(query: str = '', kind: Kind | None = None,
            limit: Annotated[StrictInt, Field(ge=1, le=20)] = 10,
            offset: Annotated[StrictInt, Field(ge=0, le=4294967295)] = 0,
            spawn_area_id: Annotated[StrictInt, Field(ge=1, le=4294967295)] | None = None) -> dict:
        """Search local Forever names by literal terms and optionally filter by cached spawn area ID.

        Omit the name query to list NPCs and objects with spawn entries in that area.
        Area IDs use the provider namespace. Results have stable pages and exact record keys.
        """
        try:
            return world.search(query, kind, limit, offset, spawn_area_id=spawn_area_id)
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
    cli.add_argument('--spawn-area-id', type=int, help='Filter search by a cached NPC or object spawn area ID')
    cli.add_argument('--limit', type=int, default=10)
    cli.add_argument('--offset', type=int, default=0)
    cli.add_argument('--include-quest-links', action='store_true', help='Include related quests, contacts and targets in get')
    cli.add_argument('--include-item-sources', action='store_true', help='Include acquisition sources or sourced items in get')
    cli.add_argument('--item-source-offset', type=int, default=0, help='Offset for each item source group in get')
    args = cli.parse_args(argv)
    try:
        if args.action != 'search' and args.spawn_area_id is not None:
            raise ValueError('Spawn area filter requires search')
        if args.action != 'get' and (args.include_quest_links or args.include_item_sources or args.item_source_offset):
            raise ValueError('Record enrichment options require get')
        if args.action == 'sync-source' and args.source is not None:
            result = sync_source(args.source)
        elif args.action == 'build' and args.source is not None and args.cache is not None:
            result = build(args.source, args.cache)
        elif args.action in ('get','search','serve') and args.cache is not None:
            world = World(args.cache)
            if args.action == 'serve':
                serve(world)
                return
            if args.action == 'get':
                result = world.get(args.kind, args.entity_id, include_quest_links=args.include_quest_links,
                                   include_item_sources=args.include_item_sources,
                                   item_source_offset=args.item_source_offset)
            else:
                result = world.search(args.query if args.query is not None else '',
                                      args.kind, args.limit, args.offset, spawn_area_id=args.spawn_area_id)
        else:
            raise ValueError('Required explicit paths unavailable')
        print(encoded(result))
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        cli.exit(1, 'Forever world operation stopped: invalid, unavailable or over-budget pinned inputs.\n')


if __name__ == '__main__':
    main()
