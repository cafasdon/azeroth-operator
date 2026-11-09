"""Explicit scan budgets admit complete larger pools without truncation or copy loss."""
from copy import deepcopy
import asyncio
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from tools import rank_forever_slot as ranker
from tools import collect_forever_gear_windows as controller
from tools import evaluate_forever_owned_gear as owned
from test_simulator_level import inputs


class CandidateScanLimitTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_upper_budget_is_a_complete_bounded_scan(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file():self.skipTest('Pinned local catalogue required; no download')
        saved=fixture();saved['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        saved['sections']['bags']={str(bag):{str(slot):{'item_id':4549,'count':1,
            'link':'item:4549:0:0:0:0:0:0'} for slot in range(1,1025)} for bag in range(4)}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryFile(mode='w+') as errors:
            key=SnapshotStore(Path(folder)).ingest(saved)
            with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}):
                drafts,skipped,_,_,_,_,search=await asyncio.wait_for(ranker.collect(
                    root/'target/debug/azeroth-operator.exe',key,['finger1'],errors,
                    collapse_identical=True,max_drafts=1,candidate_scan_limit=4096),timeout=45)
                self.assertEqual(search['candidate_scan_limit'],4096)
                self.assertEqual(len(drafts),2)
                self.assertEqual(sum(row['reason']=='equivalent_simulator_item' for row in skipped),4095)

    async def test_invalid_budgets_reject_before_io(self):
        for value in [None,True,False,0,-1,4097,1.0,'256',[],{}]:
            with self.subTest(value=value),patch.object(ranker,'stdio_client') as client:
                with self.assertRaises(ValueError):
                    await ranker.collect(None,'PRIVATE',['finger1'],None,candidate_scan_limit=value)
                client.assert_not_called()
            with self.assertRaises(ValueError):controller.validate(SimpleNamespace(candidate_scan_limit=value))
            with self.assertRaises(ValueError):await owned.execute(SimpleNamespace(candidate_scan_limit=value))
        for value in [1,256,257,4096]:self.assertEqual(ranker.validate_candidate_scan_limit(value),value)

    async def test_actual_mcp_large_identical_pool_requires_explicit_budget(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file():self.skipTest('Pinned local catalogue required; no download')
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        saved=fixture();saved['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        item={'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}
        saved['sections']['bags']={'0':{str(i):deepcopy(item) for i in range(1,258)}}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryFile(mode='w+') as errors:
            key=SnapshotStore(Path(folder)).ingest(saved)
            with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    await ranker.collect(binary,key,['finger1','finger2'],errors,collapse_identical=True,max_drafts=2)
                matching,other=caught.exception.split(ranker.GearSearchLimit)
                self.assertIsNone(other)
                def leaves(error):
                    if isinstance(error,BaseExceptionGroup):
                        for child in error.exceptions:yield from leaves(child)
                    else:yield error
                self.assertEqual(len(list(leaves(matching))),1)
                self.assertIn('candidate scan limit',str(next(leaves(matching))))
                results=[]
                for budget in [257,300]:
                    result=await asyncio.wait_for(ranker.collect(binary,key,['finger1','finger2'],errors,
                        collapse_identical=True,max_drafts=2,candidate_scan_limit=budget,
                        combination_offset=0,combination_limit=9),timeout=45)
                    drafts,skipped,_,_,_,_,search=result
                    self.assertEqual(search['candidate_scan_limit'],budget)
                    self.assertEqual(len(drafts),3)
                    self.assertEqual(sum(row['reason']=='equivalent_simulator_item' for row in skipped),510)
                    paired=next(selections for selections,_ in drafts if len(selections)==2)
                    self.assertEqual(len({json.dumps(row['source'],sort_keys=True) for row in paired}),2)
                    results.append(search)
                self.assertEqual(results[0]['combination_window']['scope_sha256'],results[1]['combination_window']['scope_sha256'])
                template=Path(folder)/'template.json';template.write_text(json.dumps(inputs()[0]))
                journal=Path(folder)/'plans.sqlite'
                def arguments(budget):
                    return controller.parser().parse_args(['--mcp',str(binary),'--snapshot-id',key,
                        '--slot','finger1','--slot','finger2','--template',str(template),'--journal',str(journal),
                        '--collapse-identical-items','--window-size','1','--max-drafts','2','--max-windows','1',
                        '--time-budget-seconds','45','--candidate-scan-limit',str(budget)])
                first=await asyncio.wait_for(controller.collect_windows(arguments(257)),timeout=50)
                second=await asyncio.wait_for(controller.collect_windows(arguments(300)),timeout=50)
                self.assertEqual(first['candidate_scan_limit'],257)
                self.assertEqual(second['candidate_scan_limit'],300)
                self.assertEqual(first['collection_binding_sha256'],second['collection_binding_sha256'])
                self.assertEqual(second['progress']['window_count'],2)
                variants=deepcopy(saved)
                for slot,row in variants['sections']['bags']['0'].items():
                    row.update(item_id=9461,link=f'item:9461:0:0:0:0:0:{13013 if slot=="257" else 13012}')
                variant_key=SnapshotStore(Path(folder)).ingest(variants)
                drafts,skipped,_,_,_,_,search=await asyncio.wait_for(ranker.collect(binary,variant_key,
                    ['finger1','finger2'],errors,collapse_identical=True,max_drafts=128,
                    candidate_scan_limit=257,combination_offset=0,combination_limit=16),timeout=45)
                self.assertEqual(search['combinations_including_baseline'],16)
                specs=[draft['checks']['simulator_equipment_spec']['items'] for _,draft in drafts]
                self.assertTrue(any(any(item.get('randomSuffix')==13013 for item in spec) for spec in specs))
                self.assertTrue(any(any(item.get('randomSuffix')==13012 for item in spec) for spec in specs))
                self.assertEqual(sum(row['reason']=='equivalent_simulator_item' for row in skipped),508)


if __name__=='__main__':unittest.main()
