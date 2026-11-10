"""Captured spell resources stay historical, exact-ID and independent of castability."""
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


class SpellResourceTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_readers_summary_exact_action_ids_and_legacy(self):
        root = Path(__file__).resolve().parents[1]
        binary = (root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture(); base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        rows = {str(slot):{'base_spell_id':a,'spell_id':b,'name':'PRIVATE_SPELL'} for slot,a,b in
            [(10,100,101),(2,100,101),(3,200,201),(4,300,301),(5,400,401)]}
        rows['10'].update(charge_count={'current':0,'maximum':2},reported_cast_count=0,
            cooldown_state={'enabled':False,'active':True})
        rows['2'].update(charge_count={'current':1,'maximum':2},reported_cast_count=5,
            cooldown_state={'enabled':True,'active':False})
        rows['3'].update(reported_cast_count=4294967295,cooldown_state={'enabled':False,'active':False})
        rows['4'].update(charge_count={'current':4294967295,'maximum':4294967295},
            cooldown_state={'enabled':True,'active':True})
        rows['2']['usability'] = {'usable':True,'insufficient_power':False}
        rows['10']['usability'] = {'usable':False,'insufficient_power':True}
        rows['4']['usability'] = {'usable':False,'insufficient_power':False}
        base['sections']['spellbook'] = {'coverage':'visible_player_spell_slots','entries':rows}
        base['sections']['action_bar_observation'] = {'coverage':'loaded_builtin_buttons_only',
            'observed_buttons':5,'missing_buttons':0,'slots':{
                str(i):{'empty':False,'kind':'spell','id':n} for i,n in enumerate([100,101,201,999,401],1)}}
        fields = ['charge_count','reported_cast_count','cooldown_state']
        invalid = []
        for charges in [None,{},[],{'current':1},{'maximum':2},{'current':0,'maximum':0},
            {'current':3,'maximum':2},{'current':-1,'maximum':2},{'current':True,'maximum':2},
            {'current':1.0,'maximum':2},{'current':1,'maximum':False},
            {'current':1,'maximum':4294967296},{'current':1,'maximum':2,'private':'PRIVATE'}]:
            invalid.append(('charge_count',charges))
        for count in [None,True,-1,1.0,4294967296,{},[], 'PRIVATE']:
            invalid.append(('reported_cast_count',count))
        for state in [None,{},[],{'enabled':True},{'enabled':0,'active':False},
            {'enabled':True,'active':None},{'enabled':True,'active':False,'private':'PRIVATE'}]:
            invalid.append(('cooldown_state',state))
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)); good = []; bad = []
            for mode in ['full','legacy','empty','missing','maximum','ambiguous','partial']:
                saved = deepcopy(base)
                if mode in ('full','ambiguous'):
                    saved['sections']['action_bar_observation']['button_hotkeys']={
                        '1':{'button':'ActionButton2','slot':2,'key1':'2','key2':'SHIFT-2'},
                        '2':{'button':'ActionButton3','slot':3,'key1':'3'},
                    }
                if mode=='legacy':
                    for row in saved['sections']['spellbook']['entries'].values():
                        for field in fields: row.pop(field,None)
                if mode=='empty': saved['sections']['spellbook']['entries'] = {}
                if mode=='missing': saved['sections'].pop('spellbook')
                if mode=='partial':
                    for field in fields: saved['sections']['spellbook']['entries']['2'].pop(field)
                if mode=='ambiguous': saved['sections']['spellbook']['entries']['6']={
                    'base_spell_id':100,'spell_id':102}
                if mode=='maximum': saved['sections']['spellbook']['entries'] = {
                    str(i):{'base_spell_id':100,'spell_id':101,'charge_count':{'current':1,'maximum':2}}
                    for i in range(1,1025)}
                good.append((store.ingest(saved),mode,saved['sections'].get('spellbook')))
            for field,value in invalid:
                saved = deepcopy(base); saved['sections']['spellbook']['entries']['2'][field] = value
                with self.assertRaises(ValueError): validate_snapshot(saved)
                raw = json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key = hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            conflicting=deepcopy(base)
            conflicting['sections']['spellbook']['entries']['10']['name']='Other label'
            conflicting_key=store.ingest(conflicting)
            placement_before=deepcopy(base)
            previous_bar=placement_before['sections']['action_bar_observation']
            previous_bar['observed_buttons']=7
            previous_bar['slots'].update({
                str(slot):{'empty':False,'kind':'spell','id':101} for slot in [6,7]})
            previous_bar['slots']['3']={'empty':False,'kind':'item','id':101}
            placement_after=deepcopy(placement_before)
            placement_after['captured_at']='2025-01-02T00:00:00Z'
            after_slots=placement_after['sections']['action_bar_observation']['slots']
            after_slots['2']={'empty':False,'kind':'item','id':101}
            after_slots['3']={'empty':False,'kind':'spell','id':101}
            del after_slots['6']
            del after_slots['7']['id']
            after_slots['8']={'empty':False,'kind':'spell','id':101}
            placement_query={'before_id':store.ingest(placement_before),
                             'after_id':store.ingest(placement_after),'action_bar_spell_id':101}
            missing_bar=deepcopy(placement_after)
            del missing_bar['sections']['action_bar_observation']
            missing_bar_key=store.ingest(missing_bar)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply = await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    conflict=await call('inspect_snapshot',snapshot_id=conflicting_key,
                        section='action_bar_observation',action_bar_spell_name='private_spell')
                    self.assertEqual(list(conflict['data']['slots']),['3','5'])
                    narrowed=await call('inspect_snapshot',snapshot_id=conflicting_key,
                        section='action_bar_observation',action_bar_spell_name='private_spell',
                        action_bar_spell_id=201,action_bar_slot=3)
                    self.assertEqual(list(narrowed['data']['slots']),['3'])
                    placement=(await call('compare_snapshots',**placement_query))[
                        'action_bar_spell_placement_change']
                    self.assertEqual(placement['before_matching_slots'],[2,6,7])
                    self.assertEqual(placement['after_matching_slots'],[3,8])
                    self.assertEqual(placement['changes'],{
                        'gained_slots':[3],'lost_slots':[2],
                        'matching_after_with_unknown_before_slots':[8],
                        'matching_before_with_unknown_after_slots':[6,7]})
                    unavailable=(await call('compare_snapshots',**{
                        **placement_query,'after_id':missing_bar_key}))['action_bar_spell_placement_change']
                    self.assertIsNone(unavailable['after_matching_slots'])
                    self.assertIsNone(unavailable['changes'])
                    both=await call('compare_snapshots',**placement_query,action_bar_item_id=101)
                    self.assertEqual(both['action_bar_spell_placement_change'],placement)
                    item=both['action_bar_item_placement_change']
                    self.assertEqual(item['item_id'],101)
                    self.assertEqual(item['before_matching_slots'],[3])
                    self.assertEqual(item['after_matching_slots'],[2])
                    self.assertEqual(item['changes'],{
                        'gained_slots':[2],'lost_slots':[3],
                        'matching_after_with_unknown_before_slots':[],
                        'matching_before_with_unknown_after_slots':[]})
                    for invalid in [
                        {'action_bar_item_id':101}, {'section':'spellbook'}, {'action_bar_spell_name':' '},
                    ]:
                        await call('inspect_snapshot',error=True,**{
                            'snapshot_id':conflicting_key,'section':'action_bar_observation',
                            'action_bar_spell_name':'private_spell',**invalid})
                    for key,mode,book in good:
                        selected = await call('inspect_snapshot',snapshot_id=key,
                            section='action_bar_observation',action_bar_spell_resource_filter={'minimum_charges':1})
                        matches = selected['action_bar_spell_resource_matches']
                        if book is None:
                            self.assertIsNone(matches)
                        else:
                            self.assertFalse(matches['cast_eligibility_verified'])
                            self.assertNotIn('100',matches['matches'])
                            expected_slots = [2] if mode in ('full','ambiguous') else list(range(1,1025)) if mode=='maximum' else None
                            expected = {} if expected_slots is None else {
                                '101':{'action_slots':[2],'matching_spellbook_slots':expected_slots,
                                       'nonmatching_spellbook_slots':[10] if mode in ('full','ambiguous') else [],
                                       'unknown_spellbook_slots':[],
                                       'observed_hotkeys_by_slot':{'2':['2','SHIFT-2']}
                                       if mode in ('full','ambiguous') else None}}
                            self.assertEqual(matches['matches'],expected)
                        if mode == 'full':
                            conflicting = await call('inspect_snapshot',snapshot_id=key,
                                section='action_bar_observation',
                                action_bar_spell_resource_filter={'minimum_charges':1,'cooldown_active':True})
                            self.assertEqual(conflicting['action_bar_spell_resource_matches']['matches'],{})
                            filtered = await call('inspect_snapshot',snapshot_id=key,
                                section='action_bar_observation',action_bar_slot=1,
                                action_bar_spell_resource_filter={'minimum_charges':1})
                            self.assertEqual(filtered['action_bar_spell_resource_matches']['matches'],{})
                            for section, predicates in [('spellbook',{'minimum_charges':1}),
                                                        ('action_bar_observation',{}),
                                                        ('action_bar_observation',{'minimum_charges':2,'maximum_charges':1})]:
                                await call('inspect_snapshot',error=True,snapshot_id=key,
                                    section=section,action_bar_spell_resource_filter=predicates)
                        named=await call('inspect_snapshot',snapshot_id=key,section='action_bar_observation',
                            action_bar_spell_name='  private_spell  ')
                        expected_slots=[] if mode in ('empty','missing','maximum') else ['2','3','5']
                        self.assertEqual(list(named['data']['slots']),expected_slots)
                        self.assertEqual(named['action_bar_spell_name_match_count'],
                            None if book is None else len(expected_slots))
                        self.assertIs(named['action_bar_spell_name_spellbook_observed'],book is not None)
                        summary = (await call('character_summary',snapshot_id=key))['spellbook_resource_observations']
                        review = (await call('inspect_snapshot',snapshot_id=key,section='action_bar_observation',action_bar_spellbook=True))['action_bar_spellbook']
                        usability = None if book is None else {}
                        if mode in ('full','legacy','ambiguous','partial'):
                            usability = {'101':[
                                {'spellbook_slot':2,'usable':True,'insufficient_power':False},
                                {'spellbook_slot':10,'usable':False,'insufficient_power':True}]}
                        self.assertEqual(review['matched_spellbook_usability_by_action_id'], usability)
                        self.assertEqual(review['spellbook_usability_conflicts_by_action_id'],
                                         None if usability is None else
                                         {'101':['usable','insufficient_power']} if usability else {})
                        self.assertEqual(review['ambiguous_base_aliases_by_action_id'],
                                         None if book is None else {'100':[101,102]} if mode=='ambiguous' else {})
                        if mode in ('full','legacy'):
                            self.assertEqual(review['observed_action_spell_slots_without_spellbook_match'],[4])
                        elif mode == 'empty':
                            self.assertEqual(review['observed_action_spell_slots_without_spellbook_match'],[1,2,3,4,5])
                        if book is None:
                            self.assertIsNone(summary)
                            self.assertIsNone(review['observed_action_spell_slots_without_spellbook_match'])
                            self.assertNotIn('matched_spellbook_resources_by_action_id',review)
                            self.assertNotIn('spellbook_resource_conflicts_by_action_id',review)
                            continue
                        values = list(book['entries'].values())
                        self.assertEqual(summary['observed_rows'],len(values))
                        for field,known,unknown in [('charge_count','known_charge_rows','unknown_charge_rows'),
                            ('reported_cast_count','known_reported_cast_count_rows','unknown_reported_cast_count_rows'),
                            ('cooldown_state','known_cooldown_state_rows','unknown_cooldown_state_rows')]:
                            count = sum(field in row for row in values)
                            self.assertEqual((summary[known],summary[unknown]),(count,len(values)-count))
                        self.assertEqual(summary['positive_charge_rows'],sum(row.get('charge_count',{}).get('current',0)>0 for row in values))
                        self.assertEqual(summary['zero_charge_rows'],sum(row.get('charge_count',{}).get('current')==0 for row in values))
                        self.assertEqual(summary['zero_reported_cast_count_rows'],sum(row.get('reported_cast_count')==0 for row in values))
                        self.assertEqual(summary['positive_reported_cast_count_rows'],sum(row.get('reported_cast_count',0)>0 for row in values))
                        self.assertEqual(summary['active_cooldown_rows'],sum(row.get('cooldown_state',{}).get('active',False) for row in values))
                        self.assertEqual(summary['enabled_cooldown_rows'],sum(row.get('cooldown_state',{}).get('enabled',False) for row in values))
                        self.assertFalse(summary['cast_eligibility_verified']); self.assertFalse(summary['live'])
                        if mode in ['legacy','empty']:
                            self.assertNotIn('matched_spellbook_resources_by_action_id',review)
                            self.assertNotIn('spellbook_resource_conflicts_by_action_id',review)
                            continue
                        conflicts = {'101':fields} if mode in ('full','ambiguous') else {}
                        self.assertEqual(review['spellbook_resource_conflicts_by_action_id'], conflicts)
                        resources = review['matched_spellbook_resources_by_action_id']
                        self.assertEqual(resources['100'],[])  # Base alias alone never carries reported-ID resources.
                        self.assertEqual(resources['999'],[])
                        for action_id,matched in resources.items():
                            wanted = [(int(slot),row) for slot,row in book['entries'].items() if row['spell_id']==int(action_id)]
                            self.assertEqual(matched,[{'spellbook_slot':slot,'base_spell_id':row['base_spell_id'],
                                'queried_spell_id':row['spell_id'],**{field:row.get(field) for field in fields},
                                'cast_eligibility_verified':False} for slot,row in sorted(wanted)])
                        if mode=='full':
                            self.assertEqual([row['spellbook_slot'] for row in resources['101']],[2,10])
                            self.assertTrue(review['spell_slots']['1']['matches_base_spell_id'])
                    for key in bad: await call('character_summary',error=True,snapshot_id=key)
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
