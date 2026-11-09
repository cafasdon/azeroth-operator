"""Optional local reference-provider captures remain strict, bounded and owner-local."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class RecipeItemSourcesTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_strict_parity_capture_bounds_and_no_private_projection(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        sources = {'coverage':'first_reported_fixed_recipe_reagents','provider':'questiedb',
            'provider_mode':'baked','provider_contract_version':4,'profession_id':164,'reported_item_count':3,
            'entries':{'2840':{'status':'reported','npc_drops':{'entries':{'1':True,'4294967295':True},
                'distinct_id_count':2,'truncated':False},'object_drops':{'entries':{},'distinct_id_count':0,'truncated':False}},
                '3470':{'status':'not_in_provider'},'9999':{'status':'unavailable'}}}
        valid = [None,sources]
        zero = deepcopy(sources); zero.update(entries={},reported_item_count=0); valid.append(zero)
        cap = deepcopy(sources)
        cap.update(provider_mode='source',reported_item_count=16384,entries={str(i):{'status':'reported'} for i in range(1,65)})
        cap['entries']['1']['vendors'] = {'entries':{str(i):True for i in range(1,9)},'distinct_id_count':4096,'truncated':True}
        valid.append(cap)
        bad = []
        for field,value in [('provider','PRIVATE'),('provider_mode','Vanilla'),('coverage','all_world'),
            ('provider_contract_version',0),('provider_contract_version',True),('profession_id',0),
            ('reported_item_count',16385),('reported_item_count',4),('extra','PRIVATE')]:
            row=deepcopy(sources); row[field]=value; bad.append(row)
        for malformed in [None,{}, {'status':'reported','vendors':None}, {'status':'reported','name':'PRIVATE'},
            {'status':'not_in_provider','vendors':{}}, {'status':'invalid'}, {'status':True},
            {'status':'reported','vendors':{'entries':{'0':True},'distinct_id_count':1,'truncated':False}},
            {'status':'reported','vendors':{'entries':{'01':True},'distinct_id_count':1,'truncated':False}},
            {'status':'reported','vendors':{'entries':{'1':1},'distinct_id_count':1,'truncated':False}},
            {'status':'reported','vendors':{'entries':{},'distinct_id_count':1,'truncated':False}},
            {'status':'reported','vendors':{'entries':{},'distinct_id_count':0,'truncated':True}},
            {'status':'reported','vendors':{'entries':{},'distinct_id_count':4097,'truncated':True}}]:
            row=deepcopy(sources); row['entries']['2840']=malformed; bad.append(row)
        row=deepcopy(sources); row['entries']['01']=row['entries'].pop('2840'); bad.append(row)
        row=deepcopy(cap); row['entries']['65']={'status':'reported'}; bad.append(row)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            good=[]; invalid=[]
            for section in valid:
                value=deepcopy(base)
                if section is not None: value['sections']['recipe_item_sources']=section
                good.append((store.ingest(value),section))
            for section,family in [(row,'forever') for row in bad]+[(sources,'retail'),(sources,'classic')]:
                value=deepcopy(base); value['client']['family']=family; value['sections']['recipe_item_sources']=section
                with self.assertRaises(ValueError): validate_snapshot(value)
                data=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(data).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(data); invalid.append(key)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,section in good:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'recipe_item_sources'})
                        self.assertFalse(reply.isError,reply)
                        result=json.loads(reply.content[0].text)
                        if section is not None: self.assertEqual(result['data'],section)
                        self.assertNotIn('PRIVATE',json.dumps(result))
                    for key in invalid:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'recipe_item_sources'})
                        self.assertTrue(reply.isError,key)
                        self.assertNotIn('PRIVATE',str(reply))
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__ == '__main__': unittest.main()
