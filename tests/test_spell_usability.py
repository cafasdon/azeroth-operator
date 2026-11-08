"""Saved API usability is historical and unknown does not match false."""
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


class SpellUsabilityTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_row_search_summary_and_readers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        rows={str(slot):{'base_spell_id':100,'spell_id':100+i,'passive':slot==10,'off_spec':False,'level_learned':20}
              for i,slot in enumerate([1,2,3,10],1)}
        rows['1']['usability']={'usable':True,'insufficient_power':False}
        rows['2']['usability']={'usable':False,'insufficient_power':True}
        rows['10']['usability']={'usable':False,'insufficient_power':False}
        value['sections']['spellbook']={'coverage':'visible_player_spell_slots','entries':rows}
        legacy=deepcopy(value)
        for row in legacy['sections']['spellbook']['entries'].values():row.pop('usability',None)
        missing=deepcopy(value);missing['sections'].pop('spellbook');missing['captured_at']='2025-01-02T00:00:00Z'
        empty=deepcopy(value);empty['sections']['spellbook']['entries']={}
        malformed=[]
        for usability in [None,{},[],{'usable':True},{'usable':True,'insufficient_power':False,'guid':'PRIVATE'}]:
            bad=deepcopy(value);bad['sections']['spellbook']['entries']['1']['usability']=usability;malformed.append(bad)
        for field in ['usable','insufficient_power']:
            for invalid in [None,0,1,'PRIVATE',[],{}]:
                bad=deepcopy(value);bad['sections']['spellbook']['entries']['1']['usability'][field]=invalid;malformed.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,legacy,missing,empty]]
            bad_keys=[]
            for bad in malformed:
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    data=await call('inspect_snapshot',snapshot_id=keys[0],section='spellbook')
                    self.assertEqual(data['data'],value['sections']['spellbook'])
                    summary=(await call('character_summary',snapshot_id=keys[0]))['spellbook_usability_observations']
                    self.assertEqual(summary,{'coverage':'visible_player_spell_slots','observed_rows':4,'known_usability_rows':3,
                        'unknown_usability_rows':1,'usable_rows':1,'unusable_rows':2,'insufficient_power_rows':1,'cast_eligibility_verified':False})
                    legacy_summary=(await call('character_summary',snapshot_id=keys[1]))['spellbook_usability_observations']
                    self.assertEqual(legacy_summary['unknown_usability_rows'],4);self.assertEqual(legacy_summary['unusable_rows'],0)
                    self.assertIsNone((await call('character_summary',snapshot_id=keys[2]))['spellbook_usability_observations'])
                    self.assertEqual((await call('character_summary',snapshot_id=keys[3]))['spellbook_usability_observations']['observed_rows'],0)
                    for options,slots in [({'usable':True},[1]),({'usable':False},[2,10]),
                            ({'insufficient_power':True},[2]),({'insufficient_power':False},[1,10]),
                            ({'usable':True,'insufficient_power':True},[]),
                            ({'usable':False,'insufficient_power':False},[10]),
                            ({'usable':False,'passive':False,'minimum_level_learned':20},[2])]:
                        result=await call('list_snapshots',family='forever',spellbook_spell={'spell_id':100,**options})
                        matches=result['snapshots'];self.assertEqual(len(matches),int(bool(slots)),options)
                        if slots:
                            self.assertEqual(matches[0]['snapshot_id'],keys[0])
                            entries=matches[0]['spellbook_spell_match']['entries']
                            self.assertEqual([row['slot'] for row in entries],slots)
                            self.assertTrue(all(isinstance(row['usability'],dict) for row in entries))
                    self.assertEqual((await call('list_snapshots',family='forever',spellbook_spell={'spell_id':103,'usable':False}))['snapshots'],[])
                    self.assertEqual((await call('list_snapshots',family='forever',latest_per_character=True,
                        spellbook_spell={'spell_id':100,'usable':True}))['snapshots'],[])
                    for options in [{'usable':1},{'usable':'true'},{'insufficient_power':0},{'insufficient_power':'false'}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','spellbook_spell':{'spell_id':100,**options}})
                        self.assertTrue(response.isError)
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'spellbook'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
