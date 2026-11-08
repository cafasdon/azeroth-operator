"""Search observed spellbook rows without inferring complete learning or usability."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def row(base,reported,**metadata):
    return {'base_spell_id':base,'spell_id':reported,'name':'PRIVATE',**metadata}


class SpellbookSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_row_filters_and_capture_selection(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));ids={}
            client={'family':'forever','variant':'live','build':'70862','interface':16001}
            def capture(label,entries,name=None,date='2025-01-01T00:00:00Z',pet=None):
                value=fixture();value['client']=client;value['character']['name']=name or label;value['captured_at']=date
                if entries is not None:value['sections']['spellbook']={'coverage':'visible_player_spell_slots','entries':entries}
                if pet is not None:value['sections']['pet_care']={'present':pet}
                ids[label]=store.ingest(value)
            capture('Mixed',{'10':row(100,101,passive=False,off_spec=True,level_learned=20),
                             '2':row(100,102,passive=True,off_spec=False,level_learned=0),'3':row(200,201)})
            capture('Reported',{'1':row(400,100,passive=False,off_spec=False,level_learned=30)},pet=True)
            capture('Zero',{'1':row(100,101,passive=False,off_spec=False,level_learned=0)})
            capture('Unknown',{'1':row(100,101)})
            capture('Many',{str(i):row(100,101,passive=False,off_spec=False,level_learned=10) for i in range(1,26)})
            capture('PartialOld',{'1':row(100,101,passive=False,off_spec=False,level_learned=20)},name='Partial')
            capture('PartialNew',{'1':row(100,101)},name='Partial',date='2025-01-02T00:00:00Z')
            capture('MixedNew',{},name='Mixed',date='2025-01-02T00:00:00Z',pet=True)
            capture('Empty',{});capture('Missing',None)
            capture('Boundary',{'4096':row(1,2**32-1,level_learned=2**32-1)})
            bad=fixture();bad['client']=client
            bad['sections']['spellbook']={'coverage':'visible_player_spell_slots','entries':{'1':row(100,101,level_learned=True)}}
            raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
            (Path(folder)/f'{hashlib.sha256(raw).hexdigest()}.json').write_bytes(raw)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(query=None,**arguments):
                        args={'family':'forever',**arguments}
                        if query is not None:args['spellbook_spell']=query
                        response=await session.call_tool('list_snapshots',args)
                        self.assertFalse(response.isError)
                        self.assertNotIn('PRIVATE',response.content[0].text)
                        return json.loads(response.content[0].text)
                    for query,names in [
                        ({'spell_id':100},['Mixed','Reported','Zero','Unknown','Many','PartialOld','PartialNew']),
                        ({'spell_id':101},['Mixed','Zero','Unknown','Many','PartialOld','PartialNew']),
                        ({'spell_id':100,'passive':False,'off_spec':False},['Reported','Zero','Many','PartialOld']),
                        ({'spell_id':100,'off_spec':False,'minimum_level_learned':20},['Reported','PartialOld']),
                        ({'spell_id':100,'minimum_level_learned':0,'maximum_level_learned':0},['Mixed','Zero']),
                        ({'spell_id':200},['Mixed']),({'spell_id':200,'passive':False},[]),
                        ({'spell_id':2**32-1,'minimum_level_learned':2**32-1},['Boundary']),({'spell_id':999},[]),
                    ]:
                        rows=(await search(query))['snapshots']
                        self.assertEqual({x['snapshot_id'] for x in rows},{ids[name] for name in names},query)
                    result=await search({'spell_id':100})
                    mixed=next(x for x in result['snapshots'] if x['snapshot_id']==ids['Mixed'])['spellbook_spell_match']
                    self.assertEqual([x['slot'] for x in mixed['entries']],[2,10])
                    many=next(x for x in result['snapshots'] if x['snapshot_id']==ids['Many'])['spellbook_spell_match']
                    self.assertEqual(many['matched_slots'],25);self.assertTrue(many['truncated'])
                    self.assertEqual([x['slot'] for x in many['entries']],list(range(1,21)))
                    self.assertTrue(all(set(x)=={'slot','base_spell_id','spell_id','passive','off_spec','level_learned'} for x in many['entries']))
                    unknown=next(x for x in result['snapshots'] if x['snapshot_id']==ids['Unknown'])['spellbook_spell_match']['entries'][0]
                    for field in ['passive','off_spec','level_learned']:self.assertIsNone(unknown[field])
                    self.assertEqual(result['invalid_count'],1)
                    self.assertTrue(all('spellbook_spell_match' not in x for x in (await search())['snapshots']))
                    page=await search({'spell_id':100},offset=1,limit=2)
                    self.assertEqual(page['snapshots'],result['snapshots'][1:3]);self.assertTrue(page['has_more'])
                    rows=(await search({'spell_id':100},active_pet={'present':True}))['snapshots']
                    self.assertEqual([x['snapshot_id'] for x in rows],[ids['Reported']])
                    for name,query,expected in [('Mixed',{'spell_id':100},[]),('Partial',{'spell_id':100},['PartialNew']),
                                                ('Partial',{'spell_id':100,'passive':False},[])]:
                        args={'character':{'name':name,'realm':'Test'},'latest_per_character':True}
                        rows=(await search(query,**args))['snapshots']
                        self.assertEqual({x['snapshot_id'] for x in rows},{ids[x] for x in expected})
                    rows=(await search({'spell_id':100},character={'name':'Mixed','realm':'Test'},latest_per_character=True,
                                       captured_through='2025-01-01T00:00:00Z'))['snapshots']
                    self.assertEqual([x['snapshot_id'] for x in rows],[ids['Mixed']])
                    for query in [{},{'spell_id':None},{'spell_id':0},{'spell_id':True},{'spell_id':2**32},
                        {'spell_id':100,'passive':'false'},{'spell_id':100,'off_spec':0},
                        {'spell_id':100,'minimum_level_learned':-1},{'spell_id':100,'maximum_level_learned':True},
                        {'spell_id':100,'minimum_level_learned':20,'maximum_level_learned':10},
                        {'spell_id':100,'maximum_level_learned':2**32},{'spell_id':100,'name':'PRIVATE'}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','spellbook_spell':query})
                        self.assertTrue(response.isError,query)
                    response=await session.call_tool('list_snapshots',{'family':'retail','spellbook_spell':{'spell_id':100}})
                    self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
