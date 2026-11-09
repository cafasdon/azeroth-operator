"""Invocation-local page reuse cannot bypass fresh owner snapshot/source checks."""
import asyncio
from contextlib import closing
from copy import deepcopy
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import rank_forever_slot as ranker
from tools import collect_forever_gear_windows as controller


def setup(folder):
    root=Path(__file__).resolve().parents[1];base=Path(folder)
    catalogue=root/'local-data/forever/catalogue.json'
    if not catalogue.is_file():raise unittest.SkipTest('Pinned local catalogue required; no download')
    local=base/'catalogue.json';local.write_bytes(catalogue.read_bytes())
    saved=fixture();saved['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
    saved['sections']['bags']={'0':{str(i):{'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'} for i in range(1,258)}}
    key=SnapshotStore(base).ingest(saved)
    template=base/'template.json';template.write_text(json.dumps(inputs()[0]))
    binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
    args=controller.parser().parse_args(['--mcp',str(binary),'--snapshot-id',key,
        '--slot','finger1','--slot','finger2','--template',str(template),'--journal',str(base/'plans.sqlite'),
        '--collapse-identical-items','--retain-equipment-specs','--window-size','1','--max-drafts','2',
        '--max-windows','2','--time-budget-seconds','60','--candidate-scan-limit','257'])
    env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(local)}
    return args,env


class OwnedPageCacheTests(unittest.IsolatedAsyncioTestCase):
    def test_serialized_bounds_and_mutation_isolation(self):
        cache=ranker.OwnedGearPageCache('a'*64,max_payload_bytes=100,max_entries=1)
        value={'items':[{'id':1}]};cache.put('one',value);value['items'][0]['id']=2
        found=cache.get('one');self.assertEqual(found['items'][0]['id'],1)
        found['items'][0]['id']=3;self.assertEqual(cache.get('one')['items'][0]['id'],1)
        size=cache.stats()['payload_bytes']
        self.assertEqual(size,len('one'.encode())+len(json.dumps({'items':[{'id':1}]},sort_keys=True,separators=(',',':')).encode()))
        cache.put('two',{'id':2});self.assertIsNone(cache.get('two'))
        cache.put('one',{'oversized':'x'*1000});self.assertEqual(cache.stats()['payload_bytes'],size)
        self.assertEqual(cache.get('one')['items'][0]['id'],1)
        self.assertTrue(all(type(value) is int for value in cache.stats().values()))
        for binding in [None,'PRIVATE','A'*64,'g'*64,[],1]:
            with self.assertRaises(ValueError):ranker.OwnedGearPageCache(binding)
        for field,values in [('max_payload_bytes',[None,True,0,32*1024*1024+1,1.0]),('max_entries',[None,True,0,1025,1.0])]:
            for value in values:
                with self.assertRaises(ValueError):ranker.OwnedGearPageCache('a'*64,**{field:value})

    async def test_actual_mcp_reuses_pages_with_identical_saved_plans(self):
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryFile(mode='w+') as errors:
            args,env=setup(folder);requests=[]
            original=ranker.ClientSession.call_tool
            async def observe(session,name,*values,**options):
                requests.append(name)
                return await original(session,name,*values,**options)
            with patch.dict(os.environ,env),patch.object(ranker.ClientSession,'call_tool',observe):
                expected=[]
                for offset in [0,1]:
                    result=await asyncio.wait_for(ranker.collect(args.mcp,args.snapshot_id,args.slot,errors,
                        collapse_identical=True,max_drafts=2,candidate_scan_limit=257,
                        combination_offset=offset,combination_limit=1),timeout=45)
                    expected.append((ranker.plan_report(result[0],True,True),result[6],result[1]))
                self.assertEqual(requests.count('review_forever_owned_gear'),24)
                requests.clear()
                report=await asyncio.wait_for(controller.collect_windows(args),timeout=65)
                self.assertEqual(requests.count('review_forever_owned_gear'),12)
                self.assertEqual(requests.count('inspect_snapshot'),2)
                self.assertGreaterEqual(requests.count('review_equipment_selection'),2)
                self.assertEqual(report['candidate_page_cache']['hits'],12)
                self.assertEqual(report['candidate_page_cache']['misses'],12)
                self.assertEqual(report['candidate_page_cache']['entries'],12)
                with closing(sqlite3.connect(args.journal)) as connection:
                    pages=[json.loads(row[0]) for row in connection.execute('SELECT report FROM windows ORDER BY start_key')]
                self.assertEqual(len(pages),2)
                for page,(planned,search,skipped) in zip(pages,expected,strict=True):
                    self.assertEqual(page['resolved_drafts'],planned['resolved_drafts'])
                    self.assertEqual(page['gear_search'],search)
                    self.assertEqual(page['skipped'],skipped)
                self.assertFalse(report['best_in_slot_verified'])
                original_init=ranker.OwnedGearPageCache.__init__
                def tiny_init(instance,binding):original_init(instance,binding,max_payload_bytes=1)
                fresh=deepcopy(args);fresh.journal=Path(folder)/'uncached.sqlite'
                requests.clear()
                with patch.object(ranker.OwnedGearPageCache,'__init__',tiny_init):
                    uncached=await asyncio.wait_for(controller.collect_windows(fresh),timeout=65)
                self.assertEqual(requests.count('review_forever_owned_gear'),24)
                self.assertEqual(uncached['candidate_page_cache']['entries'],0)
                self.assertEqual(uncached['candidate_page_cache']['hits'],0)
                self.assertEqual(uncached['progress'],report['progress'])

    async def test_changed_capture_or_catalogue_cannot_reuse_pages(self):
        for changed in ['capture','catalogue']:
            with self.subTest(changed=changed),tempfile.TemporaryDirectory() as folder:
                args,env=setup(folder);requests=[];cache_gets=[];bindings=[]
                original_call=ranker.ClientSession.call_tool
                original_append=controller.journal.append_report
                original_get=ranker.OwnedGearPageCache.get
                original_binding=controller.request_binding
                def get(instance,key):cache_gets.append(key);return original_get(instance,key)
                def binding(*values,**options):
                    result=original_binding(*values,**options);bindings.append(result);return result
                async def observe(session,name,*values,**options):
                    requests.append(name)
                    return await original_call(session,name,*values,**options)
                def append_then_change(*values,**options):
                    result=original_append(*values,**options)
                    path=Path(folder)/(args.snapshot_id+'.json') if changed=='capture' else Path(env['AZEROTH_FOREVER_CATALOGUE'])
                    path.write_bytes(path.read_bytes()+b' ')
                    return result
                with patch.dict(os.environ,env),patch.object(ranker.ClientSession,'call_tool',observe),patch.object(controller.journal,'append_report',append_then_change),patch.object(ranker.OwnedGearPageCache,'get',get),patch.object(controller,'request_binding',binding):
                    with self.assertRaises((ValueError,BaseExceptionGroup)) as caught:
                        await asyncio.wait_for(controller.collect_windows(args),timeout=65)
                self.assertEqual(controller.journal.journal_status(args.journal)['window_count'],1)
                self.assertEqual(requests.count('review_forever_owned_gear'),12)
                self.assertEqual(len(cache_gets),12)
                if changed=='capture':
                    self.assertEqual(requests.count('inspect_snapshot'),2)
                    self.assertEqual(len(set(bindings)),1)
                    def leaves(error):
                        if isinstance(error,BaseExceptionGroup):
                            for child in error.exceptions:yield from leaves(child)
                        else:yield error
                    self.assertTrue(any(str(error)=='Local MCP rejected the snapshot or request' for error in leaves(caught.exception)))
                else:
                    self.assertEqual(requests.count('inspect_snapshot'),1)
                    self.assertEqual(len(set(bindings)),2)


if __name__=='__main__':unittest.main()
