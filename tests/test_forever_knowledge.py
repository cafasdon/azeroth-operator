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
                            self.assertNotIn('catalogue_references', row)
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
                    reply=await session.call_tool('search_forever_knowledge', {
                        'query':'Minor Healing Potion','section':'consumables','limit':1,'include_references':True})
                    self.assertFalse(reply.isError,reply)
                    potion=json.loads(reply.content[0].text)['results'][0]
                    self.assertEqual(potion['record_id'],118)
                    effect=potion['catalogue_references']['references'][0]
                    self.assertEqual(effect['source_path'],'/effectIds/0')
                    self.assertEqual(effect['target_section'],'spell_effects')
                    self.assertEqual(effect['record_id'],679187)
                    expected_effect=next(r for r in raw_catalogue['spellEffects'] if r['id']==679187)
                    self.assertEqual(effect['catalogue_record'],expected_effect)
                    self.assertEqual(effect['catalogue_record_sha256'],knowledge.digest(knowledge.encoded(expected_effect).encode()))
                    rep_item=next(row for row in raw_catalogue['items'] if any('rep' in source for source in row.get('sources',[])))
                    for section, query, ids in [('items','Earthshaker',[17073]),
                            ('items','Furbolg Medicine Pouch',[16768]),('items','Thunderfury',[19019]),
                            ('npcs','Ragnaros',None),('items',rep_item['name'],[rep_item['id']])]:
                        arguments={'query':query,'section':section,'limit':1,'include_references':True}
                        if ids is not None:arguments['item_ids']=ids
                        reply=await session.call_tool('search_forever_knowledge',arguments)
                        self.assertFalse(reply.isError,reply)
                        result=json.loads(reply.content[0].text)
                        row=result['results'][0];source=row['catalogue_record'];links=row['catalogue_references']
                        self.assertFalse(links['complete_reference_graph_verified'])
                        self.assertFalse(links['current_acquisition_availability_verified'])
                        self.assertFalse(links['native_world_identity_verified'])
                        self.assertEqual(links['returned_reference_count'],len(links['references']))
                        for link in links['references']:
                            value=source
                            for token in link['source_path'].split('/')[1:]:
                                token=token.replace('~1','/').replace('~0','~')
                                value=value[int(token)] if isinstance(value,list) else value[token]
                            self.assertEqual(value,link['record_id'])
                            target=next((r for r in raw_catalogue[link['target_section']] if r['id']==value),None)
                            if target is None:
                                self.assertEqual(link['status'],'not_in_pinned_catalogue')
                                self.assertNotIn('catalogue_record',link)
                            else:
                                self.assertEqual(link['status'],'resolved')
                                self.assertEqual(link['catalogue_record'],target)
                                self.assertEqual(link['catalogue_record_sha256'],knowledge.digest(knowledge.encoded(target).encode()))
                        if ids==[17073]:
                            self.assertEqual([(r['target_section'],r['record_id']) for r in links['references']],
                                [('npcs',11982),('zones',2717)])
                        if ids==[16768]:self.assertEqual(links['references'][0]['status'],'not_in_pinned_catalogue')
                        if ids==[19019]:self.assertEqual(links['references'],[])
                        if ids==[rep_item['id']]:self.assertTrue(any(link['target_section']=='factions' for link in links['references']))
                        if section=='npcs':self.assertEqual(links['references'][0]['source_path'],'/zoneId')
                    for candidate_ids in [[], [4294967295]]:
                        reply = await session.call_tool('search_forever_knowledge', {
                            'query':'healing','section':'items','item_ids':candidate_ids,
                        })
                        self.assertFalse(reply.isError)
                        self.assertEqual(json.loads(reply.content[0].text)['results'], [])
                    suffix_base=next(row for row in raw_catalogue['items'] if row['id']==14270)
                    expected_paths=[]
                    for source_index,origin in enumerate(suffix_base.get('sources',[])):
                        for kind,value in sorted(origin.items()):
                            if isinstance(value,dict):
                                expected_paths.extend(f'/sources/{source_index}/{kind}/{field}' for field in ['npcId','zoneId','repFactionId'] if field in value)
                    expected_paths.extend(f'/randomSuffixOptions/{index}' for index in range(len(suffix_base['randomSuffixOptions'])))
                    all_references=[]
                    for offset in [0,32,64,4294967295]:
                        reply=await session.call_tool('search_forever_knowledge',{'query':"Gaea's Cloak",'section':'items',
                            'item_ids':[14270],'limit':1,'include_references':True,'reference_offset':offset})
                        self.assertFalse(reply.isError,reply)
                        row=json.loads(reply.content[0].text)['results'][0];links=row['catalogue_references']
                        self.assertEqual(links['reference_count'],len(expected_paths))
                        self.assertEqual(links['reference_offset'],offset);self.assertEqual(links['reference_limit'],32)
                        self.assertEqual([r['source_path'] for r in links['references']],expected_paths[offset:offset+32])
                        self.assertEqual(links['has_more_references'],offset+32<len(expected_paths))
                        self.assertFalse(links['owned_variant_verified'])
                        if offset<64:all_references.extend(links['references'])
                    self.assertEqual([r['source_path'] for r in all_references],expected_paths)
                    for link in all_references:
                        if link['target_section']=='random_suffixes':
                            self.assertEqual(link['relationship'],'catalogue_option_not_owned_variant')
                            self.assertEqual(link['status'],'resolved')
                            target=next(r for r in raw_catalogue['randomSuffixes'] if r['id']==link['record_id'])
                            self.assertEqual(link['catalogue_record'],target)
                            self.assertEqual(link['catalogue_record_sha256'],knowledge.digest(knowledge.encoded(target).encode()))
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
                    for value in [None,1,0,'true','false',[],{}]:
                        reply=await session.call_tool('search_forever_knowledge',{'query':'x','limit':1,'include_references':value})
                        self.assertTrue(reply.isError)
                    rejected=await session.call_tool('search_forever_knowledge',{'query':'x','limit':6,'include_references':True})
                    self.assertTrue(rejected.isError)
                    for offset in [None,True,-1,1.0,'1',4294967296,[],{}]:
                        rejected=await session.call_tool('search_forever_knowledge',{'query':'x','limit':1,
                            'include_references':True,'reference_offset':offset})
                        self.assertTrue(rejected.isError)
                    rejected=await session.call_tool('search_forever_knowledge',{'query':'x','reference_offset':1})
                    self.assertTrue(rejected.isError)
        self.assertEqual(hashlib.sha256(index.read_bytes()).hexdigest(), original)

    async def test_direct_reference_bound_and_mutation_isolation(self):
        # Exercise the pure public-row join independently of model/index loading.
        instance=object.__new__(knowledge.Knowledge)
        target={'id':1,'name':'Public NPC','zoneId':2}
        base={'id':3,'sources':[{'drop':{'npcId':1,'zoneId':99}} for _ in range(20)]}
        instance.source_records={'items:3':base,'npcs:1':target}
        result=instance.references('items:3')
        self.assertEqual(result['reference_count'],40);self.assertEqual(result['returned_reference_count'],32)
        self.assertTrue(result['references_truncated'])
        self.assertEqual(result['references'][1]['status'],'not_in_pinned_catalogue')
        result['references'][0]['catalogue_record']['name']='PRIVATE'
        self.assertEqual(instance.references('items:3')['references'][0]['catalogue_record']['name'],'Public NPC')
        base['sources']=[{'drop':{'npcId':True}}]
        invalid=instance.references('items:3')['references'][0]
        self.assertEqual(invalid['status'],'invalid_reference_id');self.assertIsNone(invalid['record_id'])
        instance.source_records['items:4']={'id':4,'randomSuffixOptions':[7]*65}
        instance.source_records['random_suffixes:7']={'id':7,'name':'Public suffix','stats':[1,2]}
        gathered=[]
        for offset in [0,32,64]:
            result=instance.references('items:4',offset)
            self.assertEqual(result['reference_count'],65)
            self.assertEqual(result['has_more_references'],offset+32<65)
            gathered.extend(result['references'])
        self.assertEqual([r['source_path'] for r in gathered],[f'/randomSuffixOptions/{i}' for i in range(65)])
        gathered[0]['catalogue_record']['stats'][0]=99
        self.assertEqual(instance.references('items:4')['references'][0]['catalogue_record']['stats'],[1,2])
        for offset in [None,True,-1,1.0,'1',4294967296]:
            with self.assertRaises(ValueError):instance.references('items:4',offset)

    async def test_pinned_consumable_effect_references(self):
        instance=object.__new__(knowledge.Knowledge)
        cache=Path(__file__).resolve().parents[1]/'local-data/forever/catalogue.json'
        if not cache.is_file():
            self.skipTest('Optional pinned catalogue required')
        raw=knowledge.read_catalogue(cache)
        instance.source_records={knowledge.record_key(section,row,field):row
            for section,(source_section,field) in knowledge.SOURCES.items() for row in raw[source_section]}
        effects={r['id']:r for r in raw['spellEffects']}
        count=0
        for row in raw['consumables']:
            links=instance.references(f"consumables:{row['id']}")['references']
            links=[link for link in links if link['target_section']=='spell_effects']
            self.assertEqual([link['record_id'] for link in links],row.get('effectIds',[]))
            for index,link in enumerate(links):
                count+=1
                self.assertEqual(link['source_path'],f'/effectIds/{index}')
                self.assertEqual(link['status'],'resolved')
                self.assertEqual(link['catalogue_record'],effects[link['record_id']])
                self.assertEqual(link['catalogue_record_sha256'],
                    knowledge.digest(knowledge.encoded(effects[link['record_id']]).encode()))
        self.assertEqual(count,83)

    async def test_effect_references_exact_namespace_pages_and_unknowns(self):
        instance=object.__new__(knowledge.Knowledge)
        instance.source_records={'consumables:1':{'id':1,'effectIds':[7]*33+[99,True,0,-1,2**32,'7',None]},
            'spell_effects:7':{'id':7,'spellId':123,'stats':[1]},
            'items:99':{'id':99,'name':'Wrong namespace'},
            'spell_effects:123':{'id':123,'spellId':7}}
        first=instance.references('consumables:1')
        second=instance.references('consumables:1',32)
        self.assertEqual(first['reference_count'],40)
        self.assertTrue(first['has_more_references']);self.assertFalse(second['has_more_references'])
        self.assertEqual([r['source_path'] for r in first['references']+second['references']],
            [f'/effectIds/{i}' for i in range(40)])
        self.assertEqual(second['references'][1]['status'],'not_in_pinned_catalogue')
        self.assertTrue(all(r['status']=='invalid_reference_id' for r in second['references'][2:]))
        first['references'][0]['catalogue_record']['stats'][0]=99
        self.assertEqual(instance.references('consumables:1')['references'][0]['catalogue_record']['stats'],[1])

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
