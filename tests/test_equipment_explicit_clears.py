"""Explicit empty desired gear slots preserve source evidence and exact layout."""
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.run_forever_sim import request_from


class ExplicitClearTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_clears_batch_and_guards(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local catalogue required; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        value = fixture()
        value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['stats'] = {'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
        value['sections']['equipment'] = {str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
            for slot,item in [(5,7950),(11,4549),(12,4550)]}
        value['sections']['bags'] = {'0': {'1': {'item_id': 7953, 'count': 1,
            'link': 'item:7953:0:0:0:0:0:0'}}}
        with tempfile.TemporaryDirectory() as folder:
            key = SnapshotStore(Path(folder)).ingest(value)
            path = Path(folder)/f'{key}.json'
            before = path.read_bytes()
            env = {**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(**extra):
                        reply = await session.call_tool('review_equipment_selection',
                            {'snapshot_id':key,'selections':[],**extra})
                        return reply, None if reply.isError else json.loads(reply.content[0].text)
                    _, baseline = await call(positional_simulator_equipment=True)
                    _, empty = await call(positional_simulator_equipment=True,clear_equipment_slots=[])
                    self.assertEqual(baseline,empty)
                    self.assertEqual(baseline['checks']['catalogue_projected_set_changes']['changes'], [])
                    _, replacement = await call(positional_simulator_equipment=True, clear_equipment_slots=[5],
                        selections=[{'target_slot':1,'source':{'kind':'bag','bag':0,'slot':1},
                                     'expected_item_id':7953}])
                    self.assertEqual(replacement['checks']['catalogue_projected_set_changes'], {
                        'changes': [{'set_id':221,'before_distinct_item_ids':[7950],
                                     'projected_distinct_item_ids':[7953]}],
                        'before_uncatalogued_equipped_slots':0,'projected_uncatalogued_equipped_slots':0,
                        'native_set_bonuses_verified':False,
                    })
                    _, cleared = await call(positional_simulator_equipment=True,clear_equipment_slots=[11])
                    spec = cleared['checks']['simulator_equipment_spec']
                    self.assertTrue(spec['preserveSlots'])
                    self.assertEqual(spec['items'][10],{})
                    self.assertEqual(spec['items'][11]['id'],4550)
                    self.assertEqual(baseline['checks']['simulator_equipment_spec']['items'][10]['id'],4549)
                    self.assertEqual(cleared['checks']['simulator_cleared_equipment_slots'],[11])
                    self.assertEqual(cleared['explicitly_cleared_equipment'],[
                        {'slot':11,'previous_item_observed':value['sections']['equipment']['11']}])
                    self.assertFalse(cleared['game_execution'])
                    self.assertFalse(cleared['live'])
                    template,_,encounter = inputs()
                    request = request_from(template,cleared,encounter)
                    self.assertEqual(request['raid']['parties'][0]['players'][0]['equipment'],spec)
                    _, batch = await call(positional_simulator_equipment=True,clear_equipment_slots=[12,11],
                        additional_drafts=[{'selections':[],'clear_equipment_slots':[12]}])
                    self.assertEqual(batch['checks']['simulator_cleared_equipment_slots'],[11,12])
                    child = batch['additional_draft_reviews'][0]
                    self.assertEqual(child['checks']['simulator_equipment_spec']['items'][10]['id'],4549)
                    self.assertEqual(child['checks']['simulator_equipment_spec']['items'][11],{})
                    for bad in [[11,11],[0],[4],[19],[1],[20],[True],['11'],list(range(18))]:
                        reply,_ = await call(positional_simulator_equipment=True,clear_equipment_slots=bad)
                        self.assertTrue(reply.isError,bad)
                    reply,_ = await call(clear_equipment_slots=[11])
                    self.assertTrue(reply.isError)
                    selection = {'target_slot':11,'source':{'kind':'equipment','slot':12},'expected_item_id':4550}
                    reply,_ = await call(positional_simulator_equipment=True,clear_equipment_slots=[11],selections=[selection])
                    self.assertTrue(reply.isError)
                    reply,_ = await call(positional_simulator_equipment=True,
                        additional_drafts=[{'selections':[],'clear_equipment_slots':[1]}])
                    self.assertTrue(reply.isError)
            self.assertEqual(path.read_bytes(),before)


if __name__ == '__main__':
    unittest.main()
