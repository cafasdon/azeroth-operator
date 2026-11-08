"""Filtered journal count comparisons never claim pets were gained or lost."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def row(owned,unowned,name='Species'):
    return {'name':name,'owned_rows':owned,'unowned_rows':unowned}


class PetViewComparisonTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_reports_only_observation_changes(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            def capture(entries,timestamp='2025-01-01T00:00:00Z',**changes):
                value=fixture()
                value['captured_at']=timestamp
                value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                if entries is not None:
                    value['sections']['pet_collection_view']={'coverage':'filtered_journal_species',
                        'displayed_rows':sum(x['owned_rows']+x['unowned_rows'] for x in entries.values()),
                        'entries':entries}
                for key,change in changes.items():value[key].update(change)
                return store.ingest(value)
            old={'2':row(0,1),'7':row(1,0),'8':row(0,1),'10':row(2,0),'30':row(1,0)}
            new={'2':row(1,0),'7':row(1,0,'Translated'),'9':row(1,0),'10':row(1,1),'40':row(0,1)}
            before=capture(old);after=capture(new)
            missing=capture(None);empty=capture({})
            renamed=deepcopy(old);renamed['7']['name']='Renamed'
            renamed_key=capture(renamed)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def compare(a,b,enabled=True):
                        args={'before_id':a,'after_id':b}
                        if enabled:args['pet_view_changes']=True
                        response=await session.call_tool('compare_snapshots',args)
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    delta=(await compare(before,after))['pet_view_changes']
                    self.assertEqual(delta,{'coverage':'filtered_journal_species','equal_filters':'unverified',
                        'collection_gains_or_losses':'unverified','before_displayed_rows':6,'after_displayed_rows':6,
                        'observed_only_before':[8,30],'observed_only_after':[9,40],
                        'observed_count_changes':[
                            {'species_id':2,'before_owned_rows':0,'after_owned_rows':1,'owned_rows_delta':1,
                             'before_unowned_rows':1,'after_unowned_rows':0,'unowned_rows_delta':-1},
                            {'species_id':10,'before_owned_rows':2,'after_owned_rows':1,'owned_rows_delta':-1,
                             'before_unowned_rows':0,'after_unowned_rows':1,'unowned_rows_delta':1}]})
                    self.assertEqual((await compare(before,renamed_key))['pet_view_changes']['observed_count_changes'],[])
                    for a,b in [(before,missing),(missing,after),(missing,missing)]:
                        self.assertIsNone((await compare(a,b))['pet_view_changes'])
                    delta=(await compare(before,empty))['pet_view_changes']
                    self.assertEqual(delta['observed_only_before'],[2,7,8,10,30])
                    self.assertEqual(delta['observed_count_changes'],[])
                    self.assertEqual((await compare(empty,empty))['pet_view_changes']['observed_only_after'],[])
                    self.assertNotIn('pet_view_changes',await compare(before,after,enabled=False))
                    retail=store.ingest(fixture())
                    for a,b in [(retail,retail),(before,capture(new,character={'name':'Other'})),
                                (before,capture(new,character={'realm':'Other'})),
                                (before,capture(new,client={'build':'different'})),
                                (capture(new,timestamp='2025-01-02T00:00:00Z'),before)]:
                        response=await session.call_tool('compare_snapshots',{'before_id':a,'after_id':b,'pet_view_changes':True})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
