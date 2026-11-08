"""Real MCP draft collection retains physical-copy constraints while deduplicating gear."""
import os
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from tools.rank_forever_slot import collect, GearSearchLimit


class GearDraftTests(unittest.IsolatedAsyncioTestCase):
    async def test_two_physical_rings_fit_two_distinct_draft_budget(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE', root/'local-data/forever/catalogue.json'))
        if not catalogue.is_file():
            self.skipTest('Requires the pinned local Forever catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY', root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['bags'] = {'0':{'1':{'item_id':4549,'count':1},'2':{'item_id':4549,'count':1}}}
            for row in value['sections']['bags']['0'].values():
                row['link'] = 'item:4549:0:0:0:0:0:0'
            key = SnapshotStore(Path(directory)).ingest(value)
            with tempfile.TemporaryFile(mode='w+') as errlog, patch.dict(os.environ, {
                    'AZEROTH_DATA_DIR':directory,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}):
                drafts, skipped, _, _, _, _, search = await collect(
                    binary,key,['finger1','finger2'],errlog,collapse_identical=True,max_drafts=2)
                self.assertEqual(len(drafts),3)
                self.assertEqual(search['distinct_simulator_gear_sets'],3)
                self.assertEqual(search['proposed_alternative_drafts'],6)
                self.assertEqual(search['equivalent_simulator_drafts'],2)
                self.assertEqual(search['max_drafts_basis'],'distinct_resolved_alternative_drafts')
                equivalent = [row for row in skipped if row['reason']=='equivalent_simulator_draft']
                self.assertEqual(len(equivalent),2)
                for row in equivalent:
                    self.assertIn(row['representative_review'],[2,3])
                paired = next(selections for selections,_ in drafts if len(selections)==2)
                self.assertEqual(len({str(row['source']) for row in paired}),2)
                original_drafts, _, _, _, _, _, original_search = await collect(
                    binary,key,['finger1','finger2'],errlog,max_drafts=6)
                self.assertEqual(len(original_drafts),5)
                self.assertEqual(original_search['equivalent_simulator_drafts'],0)
                gear_keys = lambda rows: {json.dumps(draft['checks']['simulator_equipment_spec'],sort_keys=True)
                                          for _,draft in rows}
                self.assertEqual(gear_keys(drafts),gear_keys(original_drafts))
                for collapse, limit in [(False,2),(True,1)]:
                    try:
                        await collect(binary,key,['finger1','finger2'],errlog,
                                      collapse_identical=collapse,max_drafts=limit)
                    except Exception as error:
                        while isinstance(error,ExceptionGroup) and len(error.exceptions)==1:
                            error = error.exceptions[0]
                        self.assertIsInstance(error,GearSearchLimit)
                        if collapse: self.assertIn('resolved search',str(error))
                    else:
                        self.fail('Expected bounded search rejection')
                value['sections']['bags']['0'].pop('2')
                one_key = SnapshotStore(Path(directory)).ingest(value)
                one_drafts, _, _, _, _, _, _ = await collect(
                    binary,one_key,['finger1','finger2'],errlog,collapse_identical=True,max_drafts=1)
                self.assertEqual(len(one_drafts),2)
                self.assertTrue(all(len(selections)<=1 for selections,_ in one_drafts))


if __name__ == '__main__':
    unittest.main()
