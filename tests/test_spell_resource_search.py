"""Shared resource predicates preserve same-row, unknown and newest-capture semantics."""
from copy import deepcopy
from itertools import product
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class SpellResourceSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_predicate_matrix_composition_latest_and_bounds(self):
        binary = (Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture(); base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name'] = 'Current'
        rows = {str(i):{'base_spell_id':100,'spell_id':101 if i%2 else 102,
            'name':'PRIVATE_BOOK_NAME','passive':i==5,'off_spec':i==9,'level_learned':i,
            'usability':{'usable':i%2==0,'insufficient_power':False}} for i in range(1,11)}
        for i,charges,casts,enabled,active in [(2,0,0,False,False),(3,1,2,True,False),
            (4,2,3,True,True),(5,4294967295,4294967295,False,True)]:
            rows[str(i)].update(charge_count={'current':charges,'maximum':max(2,charges)},
                reported_cast_count=casts,cooldown_state={'enabled':enabled,'active':active})
        rows['6']['charge_count'] = {'current':1,'maximum':2}
        rows['7']['reported_cast_count'] = 2
        rows['8']['cooldown_state'] = {'enabled':True,'active':False}
        rows['4']['name'] = 'Fixture Bolt'
        base['sections']['spellbook'] = {'coverage':'visible_player_spell_slots','entries':rows}
        def truth(row,criteria):
            checks=[]
            for field,key,op in [('minimum_charges',('charge_count','current'),lambda x,y:x>=y),
                ('maximum_charges',('charge_count','current'),lambda x,y:x<=y),
                ('minimum_reported_cast_count',('reported_cast_count',),lambda x,y:x>=y),
                ('maximum_reported_cast_count',('reported_cast_count',),lambda x,y:x<=y),
                ('cooldown_enabled',('cooldown_state','enabled'),lambda x,y:x==y),
                ('cooldown_active',('cooldown_state','active'),lambda x,y:x==y)]:
                if field not in criteria:continue
                value=row
                for part in key: value=value.get(part) if isinstance(value,dict) else None
                checks.append(None if value is None else op(value,criteria[field]))
            return False if False in checks else None if None in checks else True
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); key=store.ingest(base)
            variants={}
            for mode in ['legacy','empty','missing','large']:
                saved=deepcopy(base)
                if mode=='legacy':
                    saved['captured_at']='2025-01-02T00:00:00Z'
                    for row in saved['sections']['spellbook']['entries'].values():
                        for field in ['charge_count','reported_cast_count','cooldown_state']:row.pop(field,None)
                else:saved['character']['name']=mode
                if mode=='empty':saved['sections']['spellbook']['entries']={}
                if mode=='missing':saved['sections'].pop('spellbook')
                if mode=='large':saved['sections']['spellbook']['entries']={str(i):{
                    'base_spell_id':100,'spell_id':101,'charge_count':{'current':1,'maximum':2}} for i in range(1,1025)}
                variants[mode]=store.ingest(saved)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        if tool=='list_snapshots':self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def inspect(criteria,**extra):
                        return await call('inspect_snapshot',**{'snapshot_id':key,'section':'spellbook',
                            'spellbook_resource_filter':criteria,**extra})
                    tested=0
                    for a,b,c,d,e,f in product([None,0,1],[None,0,2],[None,0,2],[None,0,3],[None,False,True],[None,False,True]):
                        criteria={k:v for k,v in zip(['minimum_charges','maximum_charges','minimum_reported_cast_count',
                            'maximum_reported_cast_count','cooldown_enabled','cooldown_active'],[a,b,c,d,e,f]) if v is not None}
                        if not criteria or a is not None and b is not None and a>b or c is not None and d is not None and c>d:continue
                        tested+=1
                        wanted=sorted(int(i) for i,row in rows.items() if truth(row,criteria) is True)
                        found=await inspect(criteria)
                        self.assertEqual(sorted(map(int,found['data']['entries'])),wanted,criteria)
                        self.assertEqual(found['spellbook_resource_unknown_slots'],sum(truth(row,criteria) is None for row in rows.values()))
                        self.assertEqual(found['spellbook_resource_nonmatching_slots'],sum(truth(row,criteria) is False for row in rows.values()))
                        self.assertFalse(found['cast_eligibility_verified'])
                        listed=await call('list_snapshots',family='forever',character={'name':'Current','realm':'Test'},
                            spellbook_spell={'resources':criteria})
                        self.assertEqual([row['snapshot_id'] for row in listed['snapshots']],[key] if wanted else [])
                        if wanted:
                            projection=listed['snapshots'][0]['spellbook_spell_match']
                            self.assertIsNone(projection['spell_id'])
                            self.assertEqual([row['slot'] for row in projection['entries']],wanted)
                            for field in ['resource_unknown_slots','resource_nonmatching_slots']:
                                self.assertEqual(projection[field],found['spellbook_'+field])
                    self.assertEqual(tested,575)
                    criteria={'minimum_charges':1,'cooldown_active':True}
                    composed=await inspect(criteria,spellbook_spell_id=102,spellbook_passive=False,spellbook_off_spec=False)
                    self.assertEqual(list(composed['data']['entries']),['4'])
                    listed=await call('list_snapshots',family='forever',spellbook_spell={
                        'spell_id':102,'passive':False,'usable':True,'minimum_level_learned':3,'resources':criteria})
                    self.assertEqual([row['snapshot_id'] for row in listed['snapshots']],[key])
                    for field in ['charge_count','reported_cast_count','cooldown_state']:
                        self.assertEqual(listed['snapshots'][0]['spellbook_spell_match']['entries'][0][field],rows['4'][field])
                    newest=await call('list_snapshots',family='forever',character={'name':'Current','realm':'Test'},
                        latest_per_character=True,spellbook_spell={'resources':{'minimum_charges':1}})
                    self.assertEqual(newest['snapshots'],[])
                    large=await call('list_snapshots',family='forever',character={'name':'large','realm':'Test'},
                        spellbook_spell={'resources':{'minimum_charges':1}})
                    projection=large['snapshots'][0]['spellbook_spell_match']
                    self.assertEqual(projection['matched_slots'],1024);self.assertTrue(projection['truncated'])
                    self.assertEqual([row['slot'] for row in projection['entries']],list(range(1,21)))
                    for mode,expected in [('missing',None),('empty',0)]:
                        found=await inspect({'minimum_charges':1},snapshot_id=variants[mode])
                        self.assertEqual(found['spellbook_resource_unknown_slots'],expected)
                    for field in ['minimum_charges','minimum_reported_cast_count']:
                        found=await inspect({field:4294967295})
                        self.assertEqual(list(found['data']['entries']),['5'])
                    for bad in [{},{'cooldown_active':None},{'minimum_charges':True},{'minimum_charges':1.0},
                        {'minimum_charges':-1},{'maximum_charges':4294967296},{'cooldown_active':0},
                        {'minimum_charges':2,'maximum_charges':1},{'minimum_reported_cast_count':2,'maximum_reported_cast_count':1},
                        {'unexpected':1}]:
                        await call('inspect_snapshot',error=True,snapshot_id=key,section='spellbook',spellbook_resource_filter=bad)
                        await call('list_snapshots',error=True,family='forever',spellbook_spell={'resources':bad})
                    await call('inspect_snapshot',error=True,snapshot_id=key,section='equipment',spellbook_resource_filter={'minimum_charges':1})
                    await call('list_snapshots',error=True,family='retail',spellbook_spell={'resources':{'minimum_charges':1}})
                    await call('list_snapshots',error=True,family='forever',spellbook_spell={})
                    for name in [' ', 'é'*129, True]:
                        await call('list_snapshots',error=True,family='forever',spellbook_spell={'name':name})
                    for name,extra,expected in [
                        ('  fiXTUre Bolt ',{},[4]),
                        ('Fixture Bolt',{'spell_id':102,'resources':{'minimum_charges':2}},[4]),
                        ('Fixture Bolt',{'spell_id':101},[]),
                        ('Fixture Bolt',{'resources':{'minimum_charges':3}},[]),
                        ('Fixture',{},[]),
                    ]:
                        result=await call('list_snapshots',family='forever',
                            spellbook_spell={'name':name,**extra})
                        matched=[row['spellbook_spell_match']['entries'] for row in result['snapshots']]
                        self.assertEqual({entry['slot'] for entries in matched for entry in entries},set(expected))
                        self.assertTrue(all('name' not in entry for entries in matched for entry in entries))
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
