"""Real local-model MCP retrieval stays offline and joins only pinned public IDs."""
import asyncio
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools import forever_knowledge as knowledge
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class KnowledgeTests(unittest.IsolatedAsyncioTestCase):
    async def test_norm_validation_preserves_scalar_boundary_and_corruption_rejection(self):
        root = Path(__file__).resolve().parents[1]
        index = root / 'local-data/knowledge/index.sqlite'
        catalogue = root / 'local-data/forever/catalogue.json'
        if not index.exists() or not catalogue.exists():
            self.skipTest('Optional pinned catalogue and built index required')
        import numpy as np
        expected = knowledge.documents(catalogue)
        dimensions = knowledge.MODEL_LOCK['dimensions']
        cases = []
        for value in [0,1,1e-30,1e30,np.nan,np.inf,-np.inf,
                np.float32(.99989),np.nextafter(np.float32(.99989),np.float32(0)),
                np.nextafter(np.float32(.99989),np.float32(1)),np.float32(1.00011),
                np.nextafter(np.float32(1.00011),np.float32(1)),
                np.nextafter(np.float32(1.00011),np.float32(2))]:
            vector = np.zeros(dimensions,dtype='<f4');vector[0]=value
            cases.append(vector.tobytes())
        cases.extend([b'',b'\0',np.zeros(dimensions-1,dtype='<f4').tobytes(),
            np.zeros(dimensions+1,dtype='<f4').tobytes()])
        with closing(sqlite3.connect(':memory:')) as memory:
            with closing(sqlite3.connect(index)) as source:
                source.backup(memory)
            for payload in cases:
                try:
                    vector=np.frombuffer(payload,dtype='<f4')
                    # Fixed pre-change scalar predicate is the compatibility reference.
                    accepted=bool(vector.shape==(dimensions,) and np.isfinite(vector).all()
                        and np.isclose(np.linalg.norm(vector),1,atol=1e-4))
                except ValueError:
                    accepted=False
                memory.execute('UPDATE documents SET vector=? WHERE key=?',(payload,expected[0][0]))
                memory.execute('UPDATE integrity SET value=?',(knowledge.derived_digest(memory),))
                memory.commit()
                if accepted:
                    self.assertEqual(len(knowledge.checked_records(memory,expected)),len(expected))
                else:
                    with self.assertRaises(ValueError):knowledge.checked_records(memory,expected)

    async def test_real_model_mcp_offline_retrieval_and_exact_provenance(self):
        root = Path(__file__).resolve().parents[1]
        python = root / '.venv/Scripts/python.exe'
        catalogue = root / 'local-data/forever/catalogue.json'
        model = root / 'local-data/knowledge-model'
        index = root / 'local-data/knowledge/index.sqlite'
        if not all(path.exists() for path in (python, catalogue, model, index)):
            self.skipTest('Optional pinned local model, environment and built index required')
        original = hashlib.sha256(index.read_bytes()).hexdigest()
        expected = {row[0]: row for row in knowledge.documents(catalogue)}
        raw_catalogue = json.loads(catalogue.read_text())
        # Allow the event loop's loopback socket pair, reject external connections.
        script = (
            "import socket,runpy,sys;"
            "original=socket.socket.connect;"
            "socket.socket.connect=lambda sock,address:original(sock,address) "
            "if isinstance(address,tuple) and address[0] in ('127.0.0.1','::1') "
            "else (_ for _ in ()).throw(RuntimeError('External network forbidden'));"
            "sys.argv=['forever_knowledge','serve',*sys.argv[1:]];"
            "runpy.run_module('tools.forever_knowledge',run_name='__main__')"
        )
        arguments = ['--catalogue', str(catalogue), '--index', str(index), '--model', str(model)]
        with tempfile.TemporaryFile(mode='w+') as errors:
            async with stdio_client(StdioServerParameters(command=str(python),
                    args=['-c', script, *arguments], cwd=str(root),
                    env={**os.environ, 'HF_HUB_OFFLINE':'1', 'TRANSFORMERS_OFFLINE':'1'}), errlog=errors) as (read, write):
                async with ClientSession(read, write, read_timeout_seconds=None) as session:
                    await asyncio.wait_for(session.initialize(), 60)
                    tools = await session.list_tools()
                    self.assertEqual([tool.name for tool in tools.tools], ['search_forever_knowledge'])
                    tool = tools.tools[0]
                    self.assertTrue(tool.annotations.readOnlyHint)
                    self.assertFalse(tool.annotations.openWorldHint)
                    for query, section in [('Garb of Thero-shan', 'items'), ('restore my health', 'consumables'),
                                           ('" OR * NOT (', 'zones'), ('Inferior Stamina', 'enchants'),
                                           ('of the Eagle', 'random_suffixes'), ('437', 'spell_effects'),
                                           ('Movement', 'encounters')]:
                        reply = await asyncio.wait_for(session.call_tool('search_forever_knowledge', {
                            'query':query,'section':section,'limit':5,
                        }), 30)
                        self.assertFalse(reply.isError, reply)
                        result = json.loads(reply.content[0].text)
                        self.assertEqual(result['scope'], 'public_catalogue_only')
                        self.assertTrue(result['retrieval_only'])
                        self.assertFalse(result['live'])
                        self.assertEqual(result['catalogue_sha256'], knowledge.PIN['sha256'])
                        self.assertEqual(len(result['results']), min(5, sum(row[1] == section for row in expected.values())))
                        for row in result['results']:
                            self.assertNotIn('catalogue_record', row)
                            source = expected[row['record_key']]
                            self.assertEqual(row['section'], section)
                            self.assertEqual(row['name'], source[3])
                            self.assertEqual(row['document_sha256'], source[5])
                        if query == 'Garb of Thero-shan':
                            self.assertEqual({row['record_id'] for row in result['results'][:2]}, {7950, 7953})
                        if section == 'enchants':
                            variants = [row for row in result['results'] if row['record_id'] == 41]
                            self.assertEqual(len(variants), 2)
                            self.assertEqual(len({row['record_key'] for row in variants}), 2)
                        if section == 'encounters':
                            self.assertEqual(result['results'][0]['record_id'], 'Default/Movement')
                    for section, query in {
                        'items':'Thunderfury','consumables':'Healing Potion','npcs':'Ragnaros',
                        'zones':'Molten Core','factions':'Argent Dawn','enchants':'Inferior Stamina',
                        'random_suffixes':'of the Eagle','spell_effects':'437','encounters':'Movement',
                    }.items():
                        reply = await session.call_tool('search_forever_knowledge', {
                            'query':query,'section':section,'limit':2,'include_records':True,
                        })
                        self.assertFalse(reply.isError, reply)
                        result = json.loads(reply.content[0].text)
                        source_section = {'random_suffixes':'randomSuffixes','spell_effects':'spellEffects'}.get(section, section)
                        for row in result['results']:
                            record = row['catalogue_record']
                            self.assertIn(record, raw_catalogue[source_section])
                            canonical = json.dumps(record,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
                            self.assertEqual(row['catalogue_record_sha256'], hashlib.sha256(canonical).hexdigest())
                            if section == 'enchants':
                                self.assertTrue(row['record_key'].endswith(row['catalogue_record_sha256']))
                    binary = root / 'target/debug/azeroth-operator.exe'
                    with tempfile.TemporaryDirectory() as folder:
                        capture = fixture()
                        capture['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
                        capture['sections']['equipment'] = {'5': {'item_id':7950}}
                        capture['sections']['bags'] = {'0': {
                            '1': {'item_id':7953,'count':1}, '2': {'item_id':4294967295,'count':1},
                        }}
                        snapshot_id = SnapshotStore(Path(folder)).ingest(capture)
                        async with stdio_client(StdioServerParameters(command=str(binary),
                                env={**os.environ, 'AZEROTH_DATA_DIR':folder,
                                     'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}), errlog=errors) as (gear_read, gear_write):
                            async with ClientSession(gear_read, gear_write) as gear_session:
                                await gear_session.initialize()
                                owned = await gear_session.call_tool('review_forever_owned_gear', {'snapshot_id':snapshot_id})
                                self.assertFalse(owned.isError)
                                owned_rows = json.loads(owned.content[0].text)['items']
                                candidate_ids = sorted({row['observed']['item_id'] for row in owned_rows})
                        reply = await session.call_tool('search_forever_knowledge', {
                            'query':'Thunderfury','section':'items','item_ids':candidate_ids,'limit':5,
                        })
                        self.assertFalse(reply.isError, reply)
                        filtered = json.loads(reply.content[0].text)
                        self.assertEqual({row['record_id'] for row in filtered['results']}, {7950,7953})
                        self.assertEqual(filtered['candidate_filter'], {
                            'requested_item_id_count':3,'catalogue_matched_item_id_count':2,
                        })
                    for candidate_ids in [[], [4294967295]]:
                        reply = await session.call_tool('search_forever_knowledge', {
                            'query':'healing','section':'items','item_ids':candidate_ids,
                        })
                        self.assertFalse(reply.isError)
                        self.assertEqual(json.loads(reply.content[0].text)['results'], [])
                    for args in [{'query':''}, {'query':'x'*513}, {'query':'x','limit':21},
                                 {'query':'x','section':'snapshots'}, {'query':'x','limit':6,'include_records':True},
                                 {'query':'x','section':'items','item_ids':[7950,7950]},
                                 {'query':'x','section':'items','item_ids':[0]},
                                 {'query':'x','section':'items','item_ids':[True]},
                                 {'query':'x','section':'items','item_ids':['7950']},
                                 {'query':'x','section':'items','item_ids':[7950.0]},
                                 {'query':'x','section':'zones','item_ids':[7950]}]:
                        rejected = await session.call_tool('search_forever_knowledge', args)
                        self.assertTrue(rejected.isError)
        self.assertEqual(hashlib.sha256(index.read_bytes()).hexdigest(), original)

    async def test_index_pin_integrity_and_noop_build(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root / 'local-data/forever/catalogue.json'
        index = root / 'local-data/knowledge/index.sqlite'
        if not catalogue.exists() or not index.exists():
            self.skipTest('Optional pinned catalogue and built index required')
        with tempfile.TemporaryDirectory() as folder:
            extra_model = Path(folder) / 'model'
            extra_model.mkdir()
            (extra_model / 'added_tokens.json').write_text('{"sentinel":30522}')
            with self.assertRaisesRegex(ValueError, 'Unpinned'):
                knowledge.load_model(extra_model)
            bad_source = Path(folder) / 'source.json'
            bad_source.write_text('{"private":"SENTINEL"}')
            with self.assertRaises(ValueError):
                knowledge.documents(bad_source)
            damaged = Path(folder) / 'index.sqlite'
            damaged.write_bytes(index.read_bytes())
            with closing(sqlite3.connect(damaged)) as db:
                with db:
                    db.execute("UPDATE metadata SET value='{}'")
            with self.assertRaises(ValueError):
                knowledge.connect(damaged)
            for mutation in [
                'DROP TABLE keywords', 'DELETE FROM keywords', 'DELETE FROM keywords_idx',
                'UPDATE keywords_content SET id=-id',
                "UPDATE documents SET vector=x'00' WHERE key=(SELECT min(key) FROM documents)",
            ]:
                with closing(sqlite3.connect(':memory:')) as memory:
                    with closing(sqlite3.connect(index)) as source:
                        source.backup(memory)
                    memory.execute(mutation)
                    memory.commit()
                    with self.assertRaises((ValueError, sqlite3.Error)):
                        knowledge.checked_records(memory, knowledge.documents(catalogue))
        python = root / '.venv/Scripts/python.exe'
        if python.exists():
            before = index.read_bytes()
            result = subprocess.run([str(python), '-m', 'tools.forever_knowledge', 'build',
                '--catalogue', str(catalogue), '--index', str(index),
                '--model', str(root / 'local-data/knowledge-model')],
                cwd=root, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['status'], 'unchanged')
            self.assertEqual(index.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
