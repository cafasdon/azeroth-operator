"""Pet action records describe partial historical slots, never commands to execute."""
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


class PetActionBarTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_partial_context_counts_and_readers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['pet_action_bar']={'coverage':'readable_pet_action_slots','pet_present':True,'has_action_bar':True,'entries':{
            '1':{'kind':'command','command_token':'PET_ACTION_ATTACK','is_active':False,'range_check_performed':False},
            '2':{'kind':'spell','spell_id':101,'is_active':True,'autocast_allowed':True,'autocast_enabled':True,
                 'range_check_performed':True,'in_range':True},
            '10':{'kind':'spell','spell_id':102,'range_check_performed':True,'in_range':False}}}
        absent=deepcopy(value);absent['sections'].pop('pet_action_bar')
        no_pet=deepcopy(value);no_pet['sections']['pet_action_bar'].update(pet_present=False,has_action_bar=False,entries={})
        no_bar=deepcopy(no_pet);no_bar['sections']['pet_action_bar']['pet_present']=True
        unknown=deepcopy(value);unknown['sections']['pet_action_bar']['entries']={}
        maximum=deepcopy(value);maximum['sections']['pet_action_bar']['entries']={str(i):{
            'kind':'spell','spell_id':2**32-1,'autocast_enabled':False} for i in range(1,11)}
        malformed=[]
        for change in [lambda v:v.update(pet_present=False),lambda v:v.update(has_action_bar=False),lambda v:v.update(pet_present=None),
                lambda v:v.update(has_action_bar=1),lambda v:v.update(coverage='all_pet_actions'),lambda v:v.update(entries=[]),
                lambda v:v['entries'].update({'0':{'kind':'spell','spell_id':1}}),
                lambda v:v['entries'].update({'11':{'kind':'spell','spell_id':1}}),
                lambda v:v['entries'].update({'01':{'kind':'spell','spell_id':1}})]:
            bad=deepcopy(value);change(bad['sections']['pet_action_bar']);malformed.append(bad)
        for row in [None,{}, {'kind':'spell','spell_id':0}, {'kind':'spell','spell_id':True},
                {'kind':'spell','spell_id':2**32}, {'kind':'spell','spell_id':1,'command_token':None},
                {'kind':'command','command_token':'PRIVATE'}, {'kind':'command','command_token':'PET_'},
                {'kind':'command','command_token':'PET_ATTACK','spell_id':None},
                {'kind':'spell','spell_id':1,'range_check_performed':True},
                {'kind':'spell','spell_id':1,'in_range':False},
                {'kind':'spell','spell_id':1,'range_check_performed':False,'in_range':False},
                {'kind':'spell','spell_id':1,'name':'PRIVATE'}]:
            bad=deepcopy(value);bad['sections']['pet_action_bar']['entries']['2']=row;malformed.append(bad)
        for field in ['is_active','autocast_allowed','autocast_enabled','range_check_performed','in_range']:
            for invalid in [None,0,'PRIVATE']:
                bad=deepcopy(value);bad['sections']['pet_action_bar']['entries']['2'][field]=invalid;malformed.append(bad)
        for family in ['retail','classic']:
            bad=deepcopy(value);bad['client']['family']=family;malformed.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,absent,no_pet,no_bar,unknown,maximum]]
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
                    async def search(spell_id, **flags):
                        return (await call('list_snapshots',family='forever',
                            pet_action_spell={'spell_id':spell_id,**flags}))['snapshots']
                    active=await search(101,is_active=True,autocast_allowed=True,autocast_enabled=True,in_range=True)
                    self.assertEqual([row['snapshot_id'] for row in active],[keys[0]])
                    self.assertEqual(active[0]['pet_action_spell_match'],{
                        'matches':[{'slot':2,'observation':value['sections']['pet_action_bar']['entries']['2']}],
                        'cast_eligibility_verified':False,'live':False})
                    self.assertEqual(await search(102,autocast_enabled=False),[])
                    self.assertEqual([row['snapshot_id'] for row in await search(102,in_range=False)],[keys[0]])
                    maximum_matches=await search(2**32-1,autocast_enabled=False)
                    self.assertEqual([row['slot'] for row in maximum_matches[0]['pet_action_spell_match']['matches']],
                                     list(range(1,11)))
                    changed=deepcopy(value)
                    changed['sections']['pet_action_bar']['entries']['2']['autocast_enabled']=False
                    changed['sections']['pet_action_bar']['entries'].pop('10')
                    changed['sections']['pet_action_bar']['entries']['3']={'kind':'spell','spell_id':103}
                    async def compare(after):
                        return (await call('compare_snapshots',before_id=keys[0],after_id=after,
                            pet_action_bar_changes=True))['pet_action_bar_changes']
                    difference=await compare(store.ingest(changed))
                    self.assertEqual([row['slot'] for row in difference['slot_changes']],[2,3,10])
                    self.assertFalse(difference['slot_changes'][0]['after']['autocast_enabled'])
                    self.assertIsNone(difference['slot_changes'][1]['before'])
                    self.assertIsNone(difference['slot_changes'][2]['after'])
                    self.assertFalse(difference['same_pet_verified'])
                    self.assertEqual((await compare(keys[0]))['slot_changes'],[])
                    self.assertIsNone((await compare(keys[1]))['after_context'])
                    self.assertIsNone((await compare(keys[3]))['slot_changes'])
                    self.assertEqual(len((await compare(keys[4]))['slot_changes']),3)
                    self.assertEqual((await call('inspect_snapshot',snapshot_id=keys[0],section='pet_action_bar'))['data'],value['sections']['pet_action_bar'])
                    summary=(await call('character_summary',snapshot_id=keys[0]))['pet_action_bar_observations']
                    self.assertEqual(summary['observed_slots'],3);self.assertEqual(summary['unobserved_slots'],7)
                    self.assertEqual(summary['command_slots'],[1]);self.assertEqual(summary['spell_slots'],[2,10])
                    self.assertEqual(summary['reported_active_slots'],[2]);self.assertEqual(summary['autocast_enabled_spell_slots'],[2])
                    self.assertEqual(summary['unknown_autocast_spell_slots'],[10]);self.assertFalse(summary['cast_eligibility_verified'])
                    self.assertIsNone((await call('character_summary',snapshot_id=keys[1]))['pet_action_bar_observations'])
                    for key in keys[2:4]:
                        summary=(await call('character_summary',snapshot_id=key))['pet_action_bar_observations']
                        self.assertFalse(summary['has_action_bar']);self.assertIsNone(summary['unobserved_slots'])
                    summary=(await call('character_summary',snapshot_id=keys[4]))['pet_action_bar_observations']
                    self.assertEqual(summary['unobserved_slots'],10);self.assertEqual(summary['observed_slots'],0)
                    summary=(await call('character_summary',snapshot_id=keys[5]))['pet_action_bar_observations']
                    self.assertEqual(summary['spell_slots'],list(range(1,11)));self.assertEqual(summary['autocast_enabled_spell_slots'],[])
                    self.assertEqual(summary['unknown_autocast_spell_slots'],[])
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'pet_action_bar'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
