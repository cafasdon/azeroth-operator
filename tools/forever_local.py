"""Offline, private-use reference derived from one validated owner addon capture."""
import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import tempfile
from threading import RLock
from typing import Annotated, Literal

from shared.snapshots import SnapshotStore
from tools.sync_exports import absolute, ordinary

Kind = Literal['item', 'recipe', 'spell', 'skill']
KINDS = ('item', 'recipe', 'spell', 'skill')
FORMAT = 'azeroth-owner-reference-v1'
MAX_DATABASE = 16 * 1024 * 1024
SCHEMA = {
    'manifest': 'CREATE TABLE manifest (value TEXT NOT NULL)',
    'records': 'CREATE TABLE records (kind TEXT, id INTEGER, value TEXT NOT NULL, PRIMARY KEY(kind,id))',
    'edges': 'CREATE TABLE edges (recipe INTEGER, relation TEXT, target INTEGER, value TEXT NOT NULL, PRIMARY KEY(recipe,relation,target))',
}


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False)


def sha(value):
    return hashlib.sha256(encoded(value).encode()).hexdigest()


def uint(value, maximum=4294967295, minimum=1):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError('Invalid bounded integer')
    return value


def text(value):
    if (type(value) is not str or not 1 <= len(value.encode()) <= 256
            or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value)):
        raise ValueError('Invalid public label')
    return value


def extract(snapshot):
    """Positive field selection only; character state and selected-recipe state are omitted."""
    records, edges = {}, {}

    def record(kind, identity, fields):
        key = (kind, uint(identity))
        previous = records.setdefault(key, {})
        for field, value in fields.items():
            if field == 'observed_names':
                previous[field] = sorted(set(previous.get(field, []) + value))
                continue
            if field == 'observed_variants':
                previous[field] = sorted({encoded(v) for v in previous.get(field, []) + value})
                previous[field] = [json.loads(v) for v in previous[field]]
                continue
            if field in previous and previous[field] != value:
                raise ValueError('Contradictory observed public fields')
            previous[field] = value

    sections = snapshot['sections']
    for storage in ('equipment', 'bags', 'character_bank', 'account_bank'):
        data = sections.get(storage, {})
        for group in ([data] if storage == 'equipment' else data.values()):
            for item in group.values():
                fields = {}
                match = re.fullmatch(r'(?:\|c[0-9a-fA-F]{8}|\|cnIQ[0-9]+:)?\|Hitem:[^|]*\|h\[([^\[\]|]+)\]\|h(?:\|r)?', item.get('link', ''))
                if match:
                    fields['observed_names'] = [text(match[1])]
                variant = {key: item[key] for key in ('equip_location', 'required_level', 'sell_price_copper', 'native_stats') if key in item}
                if variant: fields['observed_variants'] = [variant]
                record('item', item['item_id'], fields)
    view = sections.get('profession_recipe_view', {})
    materials = sections.get('profession_recipe_materials', {})
    if materials:
        profession = materials['profession_id']
        record('skill', profession, {})
        for recipe_id, row in materials['entries'].items():
            if row['status'] != 'fixed_items':
                continue
            identity = int(recipe_id)
            observed = view.get('entries', {}).get(recipe_id, {}) if view.get('profession_id') == profession else {}
            if observed.get('learned') is False: raise ValueError('Conflicting known recipe observation')
            label = observed.get('name')
            record('recipe', identity, {'name': text(label)} if label else {})
            edges[(identity, 'profession', profession)] = {}
            for item_id, quantity in row['reagents'].items():
                record('item', int(item_id), {})
                edges[(identity, 'reagent', int(item_id))] = {'quantity': quantity}
            if 'output_item_id' in row:
                output = row['output_item_id']
                record('item', output, {})
                edges[(identity, 'output', output)] = row.get('base_output_quantity', {})
    for row in sections.get('spellbook', {}).get('entries', {}).values():
        fields = {key: row[key] for key in ('base_spell_id', 'passive') if key in row}
        if row.get('name'):
            fields['name'] = text(row['name'])
        record('spell', row['spell_id'], fields)
    for identity, row in sections.get('skill_lines', {}).items():
        fields = {'name': text(row['name'])}
        if 'category_id' in row:
            fields['category_id'] = row['category_id']
        record('skill', int(identity), fields)
    return ([(*key, encoded(value)) for key, value in sorted(records.items())],
            [(*key, encoded(value)) for key, value in sorted(edges.items())])


def content_digest(manifest, records, edges):
    return sha({'manifest': manifest, 'records': records, 'edges': edges})


class LocalForever:
    def __init__(self, database):
        path = ordinary(absolute(database))
        if not path.is_file() or path.stat().st_size > MAX_DATABASE:
            raise ValueError('Invalid local reference file')
        # One bounded byte snapshot: later file edits cannot change this process's answers.
        with path.open('rb') as handle:
            raw = handle.read(MAX_DATABASE + 1)
        if len(raw) > MAX_DATABASE:
            raise ValueError('Local reference exceeds byte limit')
        self.lock = RLock()
        self.connection = sqlite3.connect(':memory:', check_same_thread=False)
        try:
            self.connection.deserialize(raw)
            self.connection.execute('PRAGMA trusted_schema=OFF')
            self.connection.execute('PRAGMA query_only=ON')
            self.connection.create_function('local_contains', 2, lambda value, needle: any(needle in label.casefold() for label in (json.loads(value).get('observed_names') or [json.loads(value).get('name', '')])), deterministic=True)
            schema = self.connection.execute("SELECT name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'").fetchall()
            if dict(schema) != SCHEMA or len(schema) != len(SCHEMA):
                raise ValueError('Invalid local reference schema')
            if self.connection.execute('PRAGMA integrity_check').fetchall() != [('ok',)]:
                raise ValueError('Invalid local reference integrity')
            manifests = self.connection.execute('SELECT value FROM manifest').fetchall()
            if len(manifests) != 1:
                raise ValueError('Invalid local reference manifest')
            self.manifest = json.loads(manifests[0][0])
            expected = {'format', 'source', 'snapshot_id', 'build', 'interface', 'variant', 'captured_at',
                        'coverage', 'live', 'game_actions', 'redistribution_rights_implied', 'content_sha256'}
            if (set(self.manifest) != expected or self.manifest['format'] != FORMAT
                    or self.manifest['source'] != 'owner_addon_capture'
                    or self.manifest['coverage'] != 'incomplete_single_capture'
                    or any(self.manifest[key] is not False for key in ('live', 'game_actions', 'redistribution_rights_implied'))
                    or not re.fullmatch('[a-f0-9]{64}', self.manifest['snapshot_id'])
                    or not re.fullmatch('[a-f0-9]{64}', self.manifest['content_sha256'])):
                raise ValueError('Invalid local reference provenance')
            text(self.manifest['build']); text(self.manifest['variant']); text(self.manifest['captured_at']); uint(self.manifest['interface'])
            records = self.connection.execute('SELECT * FROM records ORDER BY kind,id').fetchall()
            edges = self.connection.execute('SELECT * FROM edges ORDER BY recipe,relation,target').fetchall()
            unsigned = {k: v for k, v in self.manifest.items() if k != 'content_sha256'}
            if content_digest(unsigned, records, edges) != self.manifest['content_sha256']:
                raise ValueError('Local reference content checksum mismatch')
            for kind, identity, payload in records:
                fields = json.loads(payload)
                allowed = {'item': {'observed_names', 'observed_variants'}, 'recipe': {'name'}, 'spell': {'name', 'base_spell_id', 'passive'}, 'skill': {'name', 'category_id'}}
                if kind not in KINDS or type(fields) is not dict or set(fields) - allowed[kind]:
                    raise ValueError('Invalid local public record')
                uint(identity)
                for key, value in fields.items():
                    if key == 'name': text(value)
                    elif key == 'observed_names':
                        if type(value) is not list or not value or value != sorted(set(value)): raise ValueError('Invalid observed item names')
                        for label in value: text(label)
                    elif key == 'observed_variants':
                        if type(value) is not list or not value: raise ValueError('Invalid item variants')
                        for variant in value:
                            if type(variant) is not dict or not variant or set(variant) - {'equip_location', 'required_level', 'sell_price_copper', 'native_stats'}: raise ValueError('Invalid item variant fields')
                            for field, observed in variant.items():
                                if field == 'equip_location':
                                    if type(observed) is not str or not re.fullmatch(r'(?:INVTYPE_[A-Z0-9_]+)?', observed): raise ValueError('Invalid equip location')
                                elif field == 'native_stats':
                                    if type(observed) is not dict or len(observed) > 128 or any(not re.fullmatch(r'(?:ITEM_MOD_[A-Z0-9_]+_SHORT|RESISTANCE[0-6]_NAME|EMPTY_SOCKET_[A-Z_]+)', stat) or type(number) not in (int, float) or not -4294967295 <= number <= 4294967295 for stat, number in observed.items()): raise ValueError('Invalid native stats')
                                else: uint(observed, 9999999999999 if field == 'sell_price_copper' else 4294967295, 0)
                    elif key == 'passive':
                        if type(value) is not bool: raise ValueError('Invalid passive field')
                    else: uint(value)
            keys = {(kind, identity) for kind, identity, _ in records}
            for recipe, relation, target, payload in edges:
                fields = json.loads(payload)
                if (relation not in ('reagent', 'output', 'profession') or type(fields) is not dict
                        or ('recipe', uint(recipe)) not in keys
                        or ('skill' if relation == 'profession' else 'item', uint(target)) not in keys
                        or (relation == 'reagent' and set(fields) != {'quantity'})
                        or (relation == 'profession' and fields)
                        or (relation == 'output' and set(fields) not in (set(), {'minimum', 'maximum'}))):
                    raise ValueError('Invalid local public edge')
                for value in fields.values(): uint(value)
                if 'minimum' in fields and fields['minimum'] > fields['maximum']:
                    raise ValueError('Invalid local output range')
        except Exception:
            self.connection.close()
            raise ValueError('Local reference unavailable or invalid') from None

    def close(self):
        with self.lock: self.connection.close()

    def query(self, sql, parameters):
        with self.lock: return self.connection.execute(sql, parameters).fetchall()

    def response(self, **fields):
        result = {**self.manifest, **fields}
        if len(encoded(result).encode()) > 64 * 1024:
            raise ValueError('Local response exceeds byte limit')
        return result

    def get(self, kind, entity_id, edge_limit=32, edge_offset=0):
        if kind not in KINDS: raise ValueError('Invalid record kind')
        uint(entity_id); uint(edge_limit, 64, 0); uint(edge_offset, minimum=0)
        rows = self.query('SELECT value FROM records WHERE kind=? AND id=?', (kind, entity_id))
        row = rows[0] if rows else None
        record = None if row is None else {'kind': kind, 'id': entity_id, 'record_key': f'local:{kind}:{entity_id}', 'fields': json.loads(row[0])}
        clause = 'recipe=?' if kind == 'recipe' else "target=? AND relation " + ("='profession'" if kind == 'skill' else "IN ('reagent','output')")
        links = self.query(f'SELECT recipe,relation,target,value FROM edges WHERE {clause} ORDER BY recipe,relation,target LIMIT ? OFFSET ?', (entity_id, edge_limit + 1, edge_offset)) if kind != 'spell' and row else []
        return self.response(found=row is not None, record=record,
            edges=[{'direction': 'outgoing' if kind == 'recipe' else 'incoming', 'relation': rel, 'target_kind': ('skill' if rel == 'profession' else 'item') if kind == 'recipe' else 'recipe', 'target_id': target if kind == 'recipe' else recipe, **json.loads(value)} for recipe, rel, target, value in links[:edge_limit]],
            edge_offset=edge_offset, has_more_edges=len(links) > edge_limit)

    def search(self, kind, query, limit=10, offset=0):
        if kind not in KINDS: raise ValueError('Invalid record kind')
        text(query); uint(limit, 20); uint(offset, minimum=0)
        query = query.strip().casefold()
        if not query or len(query.encode()) > 128: raise ValueError('Invalid local query')
        # Literal substring, never LIKE syntax, FTS syntax or caller SQL.
        rows = self.query('SELECT id,value FROM records WHERE kind=? AND local_contains(value,?) ORDER BY id LIMIT ? OFFSET ?', (kind, query, limit + 1, offset))
        return self.response(results=[{'kind': kind, 'id': identity, 'record_key': f'local:{kind}:{identity}', 'fields': json.loads(value)} for identity, value in rows[:limit]], offset=offset, has_more=len(rows) > limit)

    def recipe_dependencies(self, recipe_id, depth=4, node_budget=64, edge_budget=128):
        """Walk observed ingredients and reverse outputs; depth counts edges from the root.

        Producers are alternatives, never choices or unlock prerequisites. Completeness
        describes only the reachable stored graph, never all recipes or base materials.
        """
        uint(recipe_id); uint(depth, 8); uint(node_budget, 128); uint(edge_budget, 256)
        nodes, edges, frontier = [], [], []
        visited = set()

        def add_node(kind, identity, distance):
            rows = self.query('SELECT value FROM records WHERE kind=? AND id=?', (kind, identity))
            if not rows: return False
            visited.add((kind, identity))
            nodes.append({'kind': kind, 'id': identity, 'record_key': f'local:{kind}:{identity}',
                          'fields': json.loads(rows[0][0]), 'depth': distance})
            return True

        found = add_node('recipe', recipe_id, 0)
        # Appending to this list gives breadth-first order; each node expands once.
        for node in nodes:
            kind, identity = node['kind'], node['id']
            clause = "recipe=? AND relation='reagent'" if kind == 'recipe' else "target=? AND relation='output'"
            target_kind, target_column = ('item', 'target') if kind == 'recipe' else ('recipe', 'recipe')
            total = self.query(f'SELECT COUNT(*) FROM edges WHERE {clause}', (identity,))[0][0]
            reasons, included = [], 0
            if total and node['depth'] >= depth:
                reasons.append('depth_limit')
            elif total:
                remaining = edge_budget - len(edges)
                links = self.query(f'SELECT {target_column},value FROM edges WHERE {clause} ORDER BY {target_column} LIMIT ?', (identity, remaining + 1))
                for target, value in links:
                    if len(edges) >= edge_budget:
                        reasons.append('edge_budget')
                        break
                    if (target_kind, target) not in visited:
                        if len(nodes) >= node_budget:
                            reasons.append('node_budget')
                            break
                        add_node(target_kind, target, node['depth'] + 1)
                    edges.append({'source_key': node['record_key'], 'target_key': f'local:{target_kind}:{target}',
                                  'source_kind': kind, 'source_id': identity, 'target_kind': target_kind,
                                  'target_id': target, 'relation': 'reagent' if kind == 'recipe' else 'observed_producer',
                                  **json.loads(value)})
                    included += 1
            node['omitted_edges'] = total - included
            node['expansion'] = ('partial' if reasons else 'expanded') if total else (
                'no_observed_producer' if kind == 'item' else 'no_observed_reagents')
            if reasons:
                frontier.append({'record_key': node['record_key'], 'depth': node['depth'],
                                 'omitted_edges': total - included, 'reasons': reasons})
        return self.response(found=found, root_key=f'local:recipe:{recipe_id}', nodes=nodes, edges=edges,
            frontier=frontier, depth=depth, node_budget=node_budget, edge_budget=edge_budget,
            partial=bool(frontier), stored_graph_complete=found and not frontier,
            graph_coverage_scope='reachable_stored_recipe_ingredients_and_observed_producers',
            all_recipes_covered=False, craftability_verified=False, unlock_prerequisites_verified=False,
            ownership_checked=False, item_variant_equivalence_verified=False, plan_generated=False)


def build(store, snapshot_id, database):
    store, database = ordinary(absolute(store)), ordinary(absolute(database))
    if not store.is_dir() or database.resolve().is_relative_to(store.resolve()):
        raise ValueError('Output must be outside the owner snapshot store')
    if type(snapshot_id) is not str or not re.fullmatch('[a-f0-9]{64}', snapshot_id):
        raise ValueError('Invalid snapshot id')
    source = ordinary(store / (snapshot_id + '.json'))
    if not source.is_file(): raise ValueError('Snapshot must be an ordinary file')
    snapshot = SnapshotStore(store).read(snapshot_id)
    if snapshot['source'] != 'addon' or snapshot['client']['family'] != 'forever':
        raise ValueError('One Forever owner addon capture is necessary')
    records, edges = extract(snapshot)
    manifest = {'format': FORMAT, 'source': 'owner_addon_capture', 'snapshot_id': snapshot_id,
                'build': text(snapshot['client']['build']), 'interface': snapshot['client']['interface'], 'variant': text(snapshot['client']['variant']),
                'captured_at': snapshot['captured_at'], 'coverage': 'incomplete_single_capture',
                'live': False, 'game_actions': False, 'redistribution_rights_implied': False}
    manifest['content_sha256'] = content_digest(manifest, records, edges)
    previous = None
    if database.exists():
        existing = LocalForever(database)
        existing.close()
        previous = hashlib.sha256(database.read_bytes()).hexdigest()
    database.parent.mkdir(parents=True, exist_ok=True)
    descriptor, pending = tempfile.mkstemp(prefix='.local-reference-', dir=database.parent)
    os.close(descriptor)
    pending = Path(pending)
    try:
        with closing(sqlite3.connect(pending)) as connection, connection:
            connection.execute('PRAGMA page_size=4096')
            connection.execute(f'PRAGMA max_page_count={MAX_DATABASE // 4096}')
            connection.execute('PRAGMA temp_store=MEMORY')
            for sql in SCHEMA.values(): connection.execute(sql)
            connection.execute('INSERT INTO manifest VALUES (?)', (encoded(manifest),))
            connection.executemany('INSERT INTO records VALUES (?,?,?)', records)
            connection.executemany('INSERT INTO edges VALUES (?,?,?,?)', edges)
        staged = LocalForever(pending); staged.close()
        with pending.open('r+b') as handle: os.fsync(handle.fileno())
        ordinary(database)
        if previous is None:
            os.link(pending, database)  # Atomic no-clobber publication.
        else:
            if hashlib.sha256(database.read_bytes()).hexdigest() != previous:
                raise ValueError('Output changed during build')
            os.replace(pending, database)
    finally:
        pending.unlink(missing_ok=True)
    return {**manifest, 'status': 'built', 'record_count': len(records), 'edge_count': len(edges)}


def serve(knowledge):
    from mcp.server.fastmcp import FastMCP
    from mcp.types import ToolAnnotations
    from pydantic import Field, StrictInt
    server = FastMCP('Azeroth Owner Local Reference', instructions='Prefer these first-party saved observations for this exact capture and build. External reference companions supply missing facts only with explicit provenance. Incomplete observations never imply full knowledge, ownership, availability or redistribution rights. No live state or game actions.')
    annotations = ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False)

    @server.tool(annotations=annotations)
    def search_local_forever_records(kind: Kind, query: str, limit: Annotated[StrictInt, Field(ge=1, le=20)] = 10, offset: Annotated[StrictInt, Field(ge=0, le=4294967295)] = 0) -> dict:
        """Search public fields from one owner capture. Coverage is incomplete; no live state or game actions."""
        return knowledge.search(kind, query, limit, offset)

    @server.tool(annotations=annotations)
    def get_local_forever_record(kind: Kind, entity_id: Annotated[StrictInt, Field(ge=1, le=4294967295)], edge_limit: Annotated[StrictInt, Field(ge=0, le=64)] = 32, edge_offset: Annotated[StrictInt, Field(ge=0, le=4294967295)] = 0) -> dict:
        """Get one exact public record and bounded direct recipe edges from the configured offline database."""
        return knowledge.get(kind, entity_id, edge_limit, edge_offset)

    @server.tool(annotations=annotations)
    def get_local_forever_recipe_dependencies(recipe_id: Annotated[StrictInt, Field(ge=1, le=4294967295)], depth: Annotated[StrictInt, Field(ge=1, le=8)] = 4, node_budget: Annotated[StrictInt, Field(ge=1, le=128)] = 64, edge_budget: Annotated[StrictInt, Field(ge=1, le=256)] = 128) -> dict:
        """Walk offline observed ingredients and alternative producers. Depth counts recipe/item edge hops; budget stops expose frontier reasons. No observed producer does not prove a base material. Completeness covers only stored edges. No unlock, ownership, variant equivalence or craftability claims; no plan or game action."""
        return knowledge.recipe_dependencies(recipe_id, depth, node_budget, edge_budget)

    server.run(transport='stdio')


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('action', choices=('build', 'get', 'search', 'recipe-dependencies', 'serve'))
    cli.add_argument('--database', type=Path, required=True)
    cli.add_argument('--store', type=Path)
    cli.add_argument('--snapshot-id')
    cli.add_argument('--kind', choices=KINDS)
    cli.add_argument('--entity-id', type=int)
    cli.add_argument('--query')
    cli.add_argument('--limit', type=int, default=10)
    cli.add_argument('--edge-limit', type=int, default=32)
    cli.add_argument('--offset', type=int, default=0)
    cli.add_argument('--edge-offset', type=int, default=0)
    cli.add_argument('--depth', type=int, default=4)
    cli.add_argument('--node-budget', type=int, default=64)
    cli.add_argument('--edge-budget', type=int, default=128)
    args = cli.parse_args()
    if args.action == 'build':
        if args.store is None or args.snapshot_id is None: cli.error('build needs --store and --snapshot-id')
        result = build(args.store, args.snapshot_id, args.database)
    else:
        knowledge = LocalForever(args.database)
        try:
            if args.action == 'serve': return serve(knowledge)
            if args.action == 'recipe-dependencies':
                result = knowledge.recipe_dependencies(args.entity_id, args.depth, args.node_budget, args.edge_budget)
            else:
                result = knowledge.get(args.kind, args.entity_id, args.edge_limit, args.edge_offset) if args.action == 'get' else knowledge.search(args.kind, args.query, args.limit, args.offset)
        finally:
            knowledge.close()
    print(encoded(result))


if __name__ == '__main__':
    main()
