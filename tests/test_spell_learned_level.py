"""Spell learn levels are captured metadata, including unknown and linked changes."""
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


def spell(base,reported,level=None):
    row={'base_spell_id':base,'spell_id':reported,'name':'Spell','passive':False,'off_spec':False}
    if level is not None:row['level_learned']=level
    return row


class SpellLearnedLevelTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_inspection_and_exact_spell_metadata_comparison(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            base['sections']['spellbook']={'coverage':'visible_player_spell_slots','entries':{
                '3':spell(100,101,20),'8':spell(100,101,20),'4':spell(200,201,0),'5':spell(300,301)}}
            before=store.ingest(base)
            changed=deepcopy(base);changed['sections']['spellbook']['entries']={
                '30':spell(100,101,30),'31':spell(100,101),'4':spell(200,202,10),
                '5':spell(300,301,0),'6':spell(400,401,10)}
            changed['sections']['action_bar_observation']={
                'coverage':'loaded_builtin_buttons_only','observed_buttons':5,'missing_buttons':0,
                'slots':{str(slot):{'empty':False,'kind':kind,'id':spell_id}
                         for slot,kind,spell_id in [(1,'spell',201),(2,'spell',202),
                             (3,'spell',400),(4,'spell',200),(5,'item',401)]}}
            after=store.ingest(changed)
            renamed=deepcopy(base)
            renamed['sections']['spellbook']['entries']={str(int(slot)+50):{**row,'name':'Translated'}
                                                        for slot,row in base['sections']['spellbook']['entries'].items()}
            renamed_id=store.ingest(renamed)
            missing=deepcopy(base);missing['sections'].pop('spellbook');missing_id=store.ingest(missing)
            maximum=deepcopy(base);maximum['sections']['spellbook']['entries']['3']['level_learned']=2**32-1
            maximum_id=store.ingest(maximum)
            bad_keys=[]
            for invalid in [None,True,-1,1.5,'PRIVATE',2**32]:
                bad=deepcopy(base);bad['sections']['spellbook']['entries']['3']['level_learned']=invalid
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for identifier in [100,101]:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':before,'section':'spellbook',
                            'spellbook_spell_id':identifier})
                        self.assertFalse(response.isError)
                        rows=json.loads(response.content[0].text)['data']['entries']
                        self.assertEqual(set(rows),{'3','8'})
                        self.assertTrue(all(row['level_learned']==20 for row in rows.values()))
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':maximum_id,'section':'spellbook'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data']['entries']['3']['level_learned'],2**32-1)
                    async def compare(left,right):
                        response=await session.call_tool('compare_snapshots',{'before_id':left,'after_id':right,'spellbook':True})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)['spellbook_change']['changes']
                    changes=await compare(before,after)
                    self.assertEqual(changes['observed_level_learned_changes'],[
                        {'base_spell_id':100,'spell_id':101,'before_levels':[20],'after_levels':[None,30]},
                        {'base_spell_id':300,'spell_id':301,'before_levels':[None],'after_levels':[0]}])
                    self.assertEqual(changes['reported_spell_id_changes'],[
                        {'base_spell_id':200,'before_reported_spell_ids':[201],'after_reported_spell_ids':[202]}])
                    self.assertEqual(changes['after_action_slots_with_lost_spellbook_match'],{'1':201})
                    self.assertEqual(changes['after_action_slots_with_gained_spellbook_match'],{'2':202,'3':400})
                    self.assertIsNone((await compare(after,before))['after_action_slots_with_gained_spellbook_match'])
                    self.assertEqual((await compare(before,renamed_id))['observed_level_learned_changes'],[])
                    self.assertIsNone(await compare(before,missing_id))
                    for key in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'spellbook'})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
