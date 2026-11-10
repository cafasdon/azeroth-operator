"""Local public-catalogue keyword and vector retrieval with exact source identities."""
import argparse
from collections import deque
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import tempfile
from time import monotonic

from tools.sync_exports import ordinary
from tools.sync_forever_catalogue import PIN, MAX_BYTES, verify

ROOT = Path(__file__).resolve().parents[1]
MODEL_LOCK = json.loads((ROOT / 'clients/forever/embedding.lock.json').read_text())
SOURCES = {
    'items': ('items', 'id'),
    'npcs': ('npcs', 'id'),
    'zones': ('zones', 'id'),
    'factions': ('factions', 'id'),
    'consumables': ('consumables', 'id'),
    'enchants': ('enchants', 'effectId'),
    'random_suffixes': ('randomSuffixes', 'id'),
    'spell_effects': ('spellEffects', 'id'),
    'encounters': ('encounters', 'path'),
}
SECTIONS = tuple(SOURCES)
FORMAT = 'forever_public_knowledge_v1'


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)


def _validate_lookup(key, include_references, reference_offset, depth, nodes, edges):
    if (type(key) is not str or not key or len(key.encode()) > 512
            or type(include_references) is not bool
            or type(reference_offset) is not int or not 0 <= reference_offset <= 2**32-1
            or reference_offset and not include_references
            or type(depth) is not int or not 1 <= depth <= 3
            or type(nodes) is not int or not 1 <= nodes <= 256
            or type(edges) is not int or not 1 <= edges <= 1024
            or depth > 1 and (not include_references or reference_offset != 0)
            or depth == 1 and (nodes != 64 or edges != 256)):
        raise ValueError('Invalid public record lookup')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    path = ordinary(path)
    if not path.is_file() or path.stat().st_size > 128 * 1024 * 1024:
        raise ValueError('Invalid model file')
    result = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def read_catalogue(catalogue):
    with ordinary(catalogue.absolute()).open('rb') as handle:
        data = handle.read(MAX_BYTES + 1)
    verify(data)
    return json.loads(data)


def record_key(section, record, identity_field):
    key = f"{section}:{record[identity_field]}"
    if section == 'enchants':
        key += ':' + digest(encoded(record).encode())
    return key


def documents(catalogue):
    source = read_catalogue(catalogue)
    references = {
        kind: {row['id']: row['name'] for row in source[kind]}
        for kind in ('npcs', 'zones', 'factions')
    }
    rows = []
    for section, (source_section, identity_field) in SOURCES.items():
        for record in source[source_section]:
            identity = record[identity_field]
            name = record.get('name') or record.get('path') or f"Spell effect {identity}"
            text = [section, str(identity), name]
            if record.get('spellId'):
                text.append(f"spell {record['spellId']}")
            if record.get('itemId'):
                text.append(f"item {record['itemId']}")
            for target in record.get('targets', []):
                if target.get('target', {}).get('name'):
                    text.append(target['target']['name'])
            if record.get('setName'):
                text.append('set ' + record['setName'])
            for origin in record.get('sources', []):
                for key, value in origin.items():
                    text.append(key)
                    if isinstance(value, dict):
                        for field, kind in [('npcId', 'npcs'), ('zoneId', 'zones'), ('repFactionId', 'factions')]:
                            if value.get(field) in references[kind]:
                                text.append(references[kind][value[field]])
            body = ' '.join(text)
            key = record_key(section, record, identity_field)
            rows.append((key, section, identity, name, body, digest(body.encode())))
    if len(rows) != len({row[0] for row in rows}):
        raise ValueError('Ambiguous catalogue identity')
    return sorted(rows)


def load_model(model_path):
    model_path = ordinary(model_path.absolute())
    for path in model_path.rglob('*'):
        relative = path.relative_to(model_path)
        if relative.parts[0] == '.cache':
            continue
        ordinary(path)
        if path.is_file() and relative.as_posix() not in MODEL_LOCK['files']:
            raise ValueError('Unpinned embedding model file')
    for name, expected in MODEL_LOCK['files'].items():
        path = ordinary(model_path / name)
        if file_digest(path) != expected:
            raise ValueError('Embedding model checksum mismatch')
    os.environ['HF_HUB_OFFLINE'] = '1'
    os.environ['TRANSFORMERS_OFFLINE'] = '1'
    from sentence_transformers import SentenceTransformer
    return SentenceTransformer(str(model_path), local_files_only=True,
                               trust_remote_code=False, device='cpu')


def metadata():
    return {'format': FORMAT, 'catalogue_sha256': PIN['sha256'],
            'model_lock_sha256': digest(encoded(MODEL_LOCK).encode()),
            'dimensions': MODEL_LOCK['dimensions'], 'scope': 'public_catalogue_only'}


def connect(index, require_current=True):
    path = ordinary(index.absolute())
    if path.stat().st_size > 128 * 1024 * 1024:
        raise ValueError('Index too large')
    connection = sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=5)
    deadline = monotonic() + 10
    connection.set_progress_handler(lambda: monotonic() >= deadline, 10000)
    connection.execute('PRAGMA query_only=ON')
    connection.execute('PRAGMA trusted_schema=OFF')
    connection.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, 1024 * 1024)
    try:
        stored = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
        if (type(stored) is not dict or set(stored) != set(metadata())
                or stored['format'] != FORMAT or stored['scope'] != 'public_catalogue_only'
                or require_current and stored != metadata()):
            raise ValueError('Index provenance mismatch')
    except BaseException:
        connection.close()
        raise
    return connection


def derived_digest(database):
    checksum = hashlib.sha256()
    queries = [
        'SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name',
        'SELECT key,vector FROM documents ORDER BY key',
        'SELECT id,c0,c1 FROM keywords_content ORDER BY id',
        'SELECT id,block FROM keywords_data ORDER BY id',
        'SELECT segid,term,pgno FROM keywords_idx ORDER BY segid,term',
        'SELECT id,sz FROM keywords_docsize ORDER BY id',
        'SELECT k,v FROM keywords_config ORDER BY k',
    ]
    for query in queries:
        checksum.update(query.encode())
        for row in database.execute(query):
            checksum.update(encoded([value.hex() if isinstance(value, bytes) else value for value in row]).encode())
    return checksum.hexdigest()


def checked_records(database, expected):
    import numpy as np
    records = database.execute('SELECT key,section,record_id,name,body,sha256,vector FROM documents ORDER BY key LIMIT ?',
                               (len(expected) + 1,)).fetchall()
    if [row[:6] for row in records] != expected:
        raise ValueError('Index records differ from pinned source')
    keyword_rows = database.execute('SELECT key,body FROM keywords ORDER BY key LIMIT ?',
                                    (len(expected) + 1,)).fetchall()
    if keyword_rows != [(row[0], row[4]) for row in expected]:
        raise ValueError('Keyword records differ from pinned source')
    seal = database.execute('SELECT value FROM integrity').fetchone()
    if seal is None or seal[0] != derived_digest(database):
        raise ValueError('Derived search index checksum mismatch')
    norms = []
    for row in records:
        vector = np.frombuffer(row[6], dtype='<f4')
        if vector.shape != (MODEL_LOCK['dimensions'],) or not np.isfinite(vector).all():
            raise ValueError('Invalid index vector')
        norms.append(np.linalg.norm(vector))
    if not np.isclose(np.asarray(norms), 1, atol=1e-4).all():
        raise ValueError('Invalid index vector')
    return records


def build(catalogue, index, model_path):
    import numpy as np
    rows = documents(catalogue)
    index = ordinary(index.absolute())
    if index.exists():
        with closing(connect(index, require_current=False)) as current:
            current_metadata = json.loads(current.execute('SELECT value FROM metadata').fetchone()[0])
            if current_metadata == metadata():
                try:
                    checked_records(current, rows)
                except (ValueError, TypeError, sqlite3.Error):
                    pass
                else:
                    return {'status': 'unchanged', 'documents': len(rows), **metadata()}
    model = load_model(model_path)
    vectors = model.encode([row[4] for row in rows], batch_size=64,
                           normalize_embeddings=True, show_progress_bar=False)
    if vectors.shape != (len(rows), MODEL_LOCK['dimensions']) or not np.isfinite(vectors).all():
        raise ValueError('Invalid model output')
    index.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.knowledge-', dir=index.parent) as stage:
        temporary = Path(stage) / 'index.sqlite'
        with closing(sqlite3.connect(temporary)) as output:
            with output:
                output.execute('CREATE TABLE metadata(value TEXT NOT NULL)')
                output.execute('INSERT INTO metadata VALUES (?)', (encoded(metadata()),))
                output.execute('CREATE TABLE documents(key TEXT PRIMARY KEY,section TEXT,record_id INTEGER,name TEXT,body TEXT,sha256 TEXT,vector BLOB)')
                output.execute('CREATE VIRTUAL TABLE keywords USING fts5(key UNINDEXED,body)')
                output.executemany('INSERT INTO documents VALUES (?,?,?,?,?,?,?)',
                    [(*row, np.asarray(vector, dtype='<f4').tobytes()) for row, vector in zip(rows, vectors, strict=True)])
                output.executemany('INSERT INTO keywords VALUES (?,?)', [(row[0], row[4]) for row in rows])
            with output:
                output.execute('CREATE TABLE integrity(value TEXT NOT NULL)')
                output.execute('INSERT INTO integrity VALUES (?)', (derived_digest(output),))
        os.replace(temporary, index)
    return {'status': 'built', 'documents': len(rows), **metadata()}


class Knowledge:
    def __init__(self, catalogue, index, model_path):
        self.rows = documents(catalogue)
        source = read_catalogue(catalogue)
        self.source_records = {
            record_key(section, record, field): record
            for section, (source_section, field) in SOURCES.items()
            for record in source[source_section]
        }
        self.index = index
        self.model = load_model(model_path)

    def references(self, key, offset=0):
        """One-hop exact public foreign keys; absence is not a world-data claim."""
        if type(offset) is not int or not 0 <= offset <= 2**32-1:
            raise ValueError('Invalid public reference offset')
        source = self.source_records[key]
        links = []
        fields = [('npcId', 'npcs'), ('zoneId', 'zones'), ('repFactionId', 'factions')]
        if 'zoneId' in source:
            links.append(('/zoneId', 'zones', source['zoneId']))
        for index, origin in enumerate(source.get('sources', [])):
            for kind, value in sorted(origin.items()):
                if not isinstance(value, dict):
                    continue
                kind = kind.replace('~', '~0').replace('/', '~1')
                for field, section in fields:
                    if field in value:
                        links.append((f'/sources/{index}/{kind}/{field}', section, value[field]))
        for index, identity in enumerate(source.get('randomSuffixOptions', [])):
            links.append((f'/randomSuffixOptions/{index}', 'random_suffixes', identity))
        for index, identity in enumerate(source.get('effectIds', [])):
            links.append((f'/effectIds/{index}', 'spell_effects', identity))
        results = []
        for path, section, identity in links[offset:offset+32]:
            row = {'source_path': path, 'target_section': section, 'record_id': identity}
            if section == 'random_suffixes':
                row['relationship'] = 'catalogue_option_not_owned_variant'
            if type(identity) is not int or not 1 <= identity <= 2**32-1:
                row['record_id'] = None
                row['status'] = 'invalid_reference_id'
            else:
                target_key = f'{section}:{identity}'
                target = self.source_records.get(target_key)
                if target is None:
                    row['status'] = 'not_in_pinned_catalogue'
                else:
                    record = encoded(target)
                    row.update(status='resolved', record_key=target_key,
                               catalogue_record=json.loads(record), catalogue_record_sha256=digest(record.encode()))
            results.append(row)
        return {'scope': 'recognised_direct_public_fields_only', 'reference_count': len(links),
                'returned_reference_count': len(results), 'references_truncated': len(results) < len(links),
                'reference_offset': offset, 'reference_limit': 32, 'has_more_references': offset+len(results) < len(links),
                'references': results, 'complete_reference_graph_verified': False,
                'current_acquisition_availability_verified': False, 'native_world_identity_verified': False,
                'owned_variant_verified': False}

    def search(self, query, section=None, limit=10, include_records=False, item_ids=None, include_references=False,
               reference_offset=0):
        import numpy as np
        if (not isinstance(query, str) or not query.strip() or len(query.encode()) > 512
                or section is not None and section not in SECTIONS
                or type(limit) is not int or not 1 <= limit <= 20
                or type(include_records) is not bool or type(include_references) is not bool
                or (include_records or include_references) and limit > 5
                or type(reference_offset) is not int or not 0 <= reference_offset <= 2**32-1
                or reference_offset and not include_references):
            raise ValueError('Invalid public knowledge query')
        if item_ids is not None and (
                section != 'items' or type(item_ids) is not list or len(item_ids) > 10000
                or any(type(item) is not int or not 1 <= item <= 2**32 - 1 for item in item_ids)
                or len(set(item_ids)) != len(item_ids)):
            raise ValueError('Expected distinct positive candidate item IDs for the items section')
        candidates = set(item_ids) if item_ids is not None else None
        query = query.strip()
        result = {**metadata(), 'model_revision': MODEL_LOCK['revision'], 'source_revision': PIN['revision'],
                  'retrieval_only': True, 'live': False, 'results': []}
        with closing(connect(self.index)) as database:
            records = checked_records(database, self.rows)
            selected = [row for row in records if (section is None or row[1] == section)
                        and (candidates is None or row[2] in candidates)]
            if candidates is not None:
                result['candidate_filter'] = {
                    'requested_item_id_count': len(candidates), 'catalogue_matched_item_id_count': len(selected),
                }
            if not selected:
                return result
            vectors = np.stack([np.frombuffer(row[6], dtype='<f4') for row in selected])
            if vectors.shape != (len(selected), MODEL_LOCK['dimensions']) or not np.isfinite(vectors).all():
                raise ValueError('Invalid index vectors')
            vector = self.model.encode([query], normalize_embeddings=True, show_progress_bar=False)[0]
            scores = vectors @ vector
            semantic = sorted(range(len(selected)), key=lambda i: (-float(scores[i]), selected[i][0]))[:100]
            terms = re.findall(r'\w+', query, flags=re.UNICODE)
            lexical = []
            if terms:
                expression = ' OR '.join('"' + term + '"' for term in terms)
                candidate_sql = ''
                parameters = [expression, section, section]
                if candidates is not None:
                    candidate_sql = ' AND documents.record_id IN (' + ','.join('?' for _ in candidates) + ')'
                    parameters.extend(sorted(candidates))
                lexical = database.execute(
                    'SELECT keywords.key FROM keywords JOIN documents ON documents.key=keywords.key '
                    'WHERE keywords MATCH ? AND (? IS NULL OR documents.section=?) '
                    + candidate_sql + ' ORDER BY bm25(keywords),keywords.key LIMIT 100', parameters).fetchall()
        ranks = {}
        for rank, index in enumerate(semantic, 1):
            ranks[selected[index][0]] = 1 / (60 + rank)
        for rank, (key,) in enumerate(lexical, 1):
            ranks[key] = ranks.get(key, 0) + 1 / (60 + rank)
        by_key = {row[0]: row for row in selected}
        winners = sorted(ranks, key=lambda key: (-ranks[key], key))[:limit]
        result['results'] = [
                    {'section': by_key[key][1], 'record_id': by_key[key][2], 'record_key':key,
                     'name': by_key[key][3],
                     'document_sha256': by_key[key][5], 'retrieval_score': ranks[key]}
                    for key in winners]
        if include_records or include_references:
            for row in result['results']:
                source = encoded(self.source_records[row['record_key']])
                row['catalogue_record'] = json.loads(source)
                row['catalogue_record_sha256'] = digest(source.encode())
                if include_references:
                    row['catalogue_references'] = self.references(row['record_key'], reference_offset)
            if len(encoded(result).encode()) > 256 * 1024:
                raise ValueError('Source record response exceeds byte limit')
        return result

    def _reference_walk(self, key, depth, node_budget, edge_budget):
        """Called only after index validation; recognised fields, never inferred edges."""
        deadline = monotonic() + 5
        def node(record_key, distance):
            section = record_key.split(':', 1)[0]
            source = self.source_records[record_key]
            identity = source[SOURCES[section][1]]
            return {'record_key': record_key, 'section': section, 'record_id': identity,
                'name': source.get('name') or source.get('path') or f'Spell effect {identity}',
                'depth': distance, 'catalogue_record_sha256': digest(encoded(source).encode())}
        nodes = {key: node(key, 0)}
        pending = deque([(key, 0)])
        edges, expanded = [], []
        stopped = None
        while pending and stopped is None:
            source_key, distance = pending.popleft()
            if distance == depth:
                continue
            offset = 0
            while stopped is None:
                if monotonic() >= deadline:
                    stopped = 'analysis_time_budget_exhausted'
                    break
                page = self.references(source_key, offset)
                for reference in page['references']:
                    if monotonic() >= deadline:
                        stopped = 'analysis_time_budget_exhausted'
                        break
                    if len(edges) == edge_budget:
                        stopped = 'edge_budget_exhausted'
                        break
                    identity = reference['record_id']
                    target = reference.get('record_key')
                    if target is None and type(identity) is int and 1 <= identity <= 2**32-1:
                        target = f"{reference['target_section']}:{identity}"
                    edge = {'source_record_key': source_key, 'source_path': reference['source_path'],
                        'target_section': reference['target_section'], 'record_id': identity,
                        'status': reference['status'], 'target_record_key': target,
                        'target_node_included': False}
                    if 'relationship' in reference:
                        edge['relationship'] = reference['relationship']
                    if reference['status'] == 'resolved':
                        if target not in nodes:
                            if len(nodes) == node_budget:
                                edges.append(edge)
                                stopped = 'node_budget_exhausted'
                                break
                            nodes[target] = node(target, distance+1)
                            pending.append((target, distance+1))
                        edge['target_node_included'] = True
                    edges.append(edge)
                if stopped is not None or not page['has_more_references']:
                    break
                offset += len(page['references'])
            if stopped is None:
                expanded.append(source_key)
        result = {'scope': 'recognised_pinned_public_reference_fields_only',
            'status': stopped or 'complete_within_requested_depth',
            'traversal_complete_within_requested_depth': stopped is None,
            'depth_limit': depth, 'node_budget': node_budget, 'edge_budget': edge_budget,
            'analysis_time_budget_seconds': 5, 'node_count': len(nodes), 'edge_count': len(edges),
            'nodes': list(nodes.values()), 'edges': edges, 'expanded_record_keys': expanded,
            'unexpanded_discovered_record_keys': [record_key for record_key, row in nodes.items()
                if row['depth'] < depth and record_key not in expanded],
            'depth_boundary_record_keys': [record_key for record_key, row in nodes.items() if row['depth'] == depth],
            'unresolved_reference_count': sum(edge['status'] != 'resolved' for edge in edges),
            'complete_world_graph_verified': False, 'current_acquisition_availability_verified': False,
            'native_world_identity_verified': False, 'owned_variant_verified': False,
            'routing_verified': False, 'game_execution': False, 'live': False}
        if stopped is None and monotonic() >= deadline:
            result.update(status='analysis_time_budget_exhausted', traversal_complete_within_requested_depth=False)
        return result

    def get_record(self, key, include_references=False, reference_offset=0, *,
                   reference_depth=1, reference_node_budget=64, reference_edge_budget=256):
        """Follow an exact returned public record key, without similarity fallback."""
        _validate_lookup(key, include_references, reference_offset, reference_depth,
                         reference_node_budget, reference_edge_budget)
        with closing(connect(self.index)) as database:
            records = checked_records(database, self.rows)
        row = next((row for row in records if row[0] == key), None)
        result = {**metadata(), 'source_revision': PIN['revision'],
                  'retrieval_only': True, 'live': False, 'found': row is not None, 'record': None}
        if row is not None:
            source = encoded(self.source_records[key])
            record = {'section': row[1], 'record_id': row[2], 'record_key': key,
                      'name': row[3], 'document_sha256': row[5],
                      'catalogue_record': json.loads(source), 'catalogue_record_sha256': digest(source.encode())}
            if include_references:
                if reference_depth == 1:
                    record['catalogue_references'] = self.references(key, reference_offset)
                else:
                    record['catalogue_reference_walk'] = self._reference_walk(key, reference_depth,
                        reference_node_budget, reference_edge_budget)
            result['record'] = record
        if len(encoded(result).encode()) > 256 * 1024:
            raise ValueError('Source record response exceeds byte limit')
        return result


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('action', choices=('build', 'search', 'get', 'serve'))
    cli.add_argument('--catalogue', type=Path, required=True)
    cli.add_argument('--index', type=Path, required=True)
    cli.add_argument('--model', type=Path, required=True)
    cli.add_argument('--query')
    cli.add_argument('--record-key')
    cli.add_argument('--section', choices=SECTIONS)
    cli.add_argument('--limit', type=int, default=10)
    cli.add_argument('--include-records', action='store_true')
    cli.add_argument('--include-references', action='store_true')
    cli.add_argument('--reference-offset', type=int, default=0)
    cli.add_argument('--reference-depth', type=int, default=1)
    cli.add_argument('--reference-node-budget', type=int, default=64)
    cli.add_argument('--reference-edge-budget', type=int, default=256)
    cli.add_argument('--item-id', action='append', type=int, dest='item_ids')
    args = cli.parse_args()
    if args.action != 'get' and (args.reference_depth != 1 or args.reference_node_budget != 64
                                or args.reference_edge_budget != 256):
        cli.error('Reference walk options require the get action')
    if args.action == 'build':
        print(encoded(build(args.catalogue, args.index, args.model)))
        return
    if args.action == 'get':
        _validate_lookup(args.record_key, args.include_references, args.reference_offset,
                         args.reference_depth, args.reference_node_budget, args.reference_edge_budget)
    knowledge = Knowledge(args.catalogue, args.index, args.model)
    if args.action == 'get':
        print(encoded(knowledge.get_record(args.record_key, args.include_references, args.reference_offset,
            reference_depth=args.reference_depth, reference_node_budget=args.reference_node_budget,
            reference_edge_budget=args.reference_edge_budget)))
        return
    if args.action == 'search':
        print(encoded(knowledge.search(args.query, args.section, args.limit, args.include_records, args.item_ids,
                                       args.include_references, args.reference_offset)))
        return
    from mcp.server.fastmcp import FastMCP
    from mcp.types import ToolAnnotations
    from pydantic import Field, StrictBool, StrictInt
    from typing import Annotated
    server = FastMCP('Azeroth Public Knowledge')

    @server.tool(annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False))
    def get_forever_knowledge_record(record_key: str, include_references: StrictBool = False,
                                     reference_offset: StrictInt = 0,
                                     reference_depth: Annotated[StrictInt, Field(ge=1, le=3)] = 1,
                                     reference_node_budget: Annotated[StrictInt, Field(ge=1, le=256)] = 64,
                                     reference_edge_budget: Annotated[StrictInt, Field(ge=1, le=1024)] = 256) -> dict:
        """Get an exact pinned public record and SHA-256; unknown keys return found false. include_references defaults to a paged 32-edge direct preview. reference_depth 2 or 3 instead returns catalogue_reference_walk: bounded breadth-first exact links, node identities/hashes, explicit missing targets and partial-budget status; requires include_references and offset zero. Walk node/edge budgets apply only at depth above one; five-second analysis bound. No inferred joins, private snapshots, ownership, route or current availability claim. Replies capped at 256 KiB without silent truncation."""
        try:
            return knowledge.get_record(record_key, include_references, reference_offset,
                reference_depth=reference_depth, reference_node_budget=reference_node_budget,
                reference_edge_budget=reference_edge_budget)
        except (OSError, ValueError, sqlite3.Error, KeyError, TypeError):
            raise ValueError('Public knowledge index or query unavailable or invalid') from None

    @server.tool(annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False))
    def search_forever_knowledge(query: str, section: str | None = None, limit: int = 10,
                                 include_records: bool = False, item_ids: list[StrictInt] | None = None,
                                 include_references: StrictBool = False, reference_offset: StrictInt = 0) -> dict:
        """Search pinned public references. Limit 1–20, or 1–5 with include_records to attach exact source rows and hashes. include_references also attaches the base row and up to 32 direct NPC/zone/faction, listed random-suffix-option and spell-effect-ID edges per hit, with exact rows or explicit missing status. reference_offset (uint32) pages these edges and requires include_references when nonzero; no recursive or guessed links. Source-bearing replies are capped at 256 KiB. For items only, item_ids restricts retrieval to up to 10,000 distinct positive candidate IDs; an empty list returns no results. Supplied IDs and suffix options do not verify ownership, exact modifiers or equip legality."""
        try:
            return knowledge.search(query, section, limit, include_records, item_ids, include_references, reference_offset)
        except (OSError, ValueError, sqlite3.Error, KeyError, TypeError):
            raise ValueError('Public knowledge index or query unavailable or invalid') from None

    server.run(transport='stdio')


if __name__ == '__main__':
    main()
