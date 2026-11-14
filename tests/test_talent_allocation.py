"""Complete queried-entry ranks reach strict readers and simulator preparation."""
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
from tools.forever_talent_identity import talent_map
from tools import run_forever_sim as adapter
from tools import positional_equipment as layout
from test_simulator_level import inputs


def sample(name='hunter',ranks=None):
    source=talent_map()['classes'][name];value=fixture()
    value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
    value['sections']['stats']={'class':name.upper(),'race':'Orc','level':60,'client_version':'synthetic'}
    nodes={};definitions={};digits=[];offset=0
    for tree_index,size in enumerate(source['tree_sizes']):
        part=''
        for position in range(size):
            index=offset+position;entry=source['talents'][index];node=len(source['talents'])-index
            rank=(index% (entry['max_rank']+1)) if ranks is None else ranks[index]
            row={'node_id':node,'entry_id':201+index,'tree_id':10+tree_index,
                 'purchased_rank':rank,'max_rank':entry['max_rank']}
            definitions[str(entry['definition_id'])]=row
            if rank or index==0:
                nodes[str(node)]={'entry_id':row['entry_id'],'tree_id':row['tree_id'],'rank':rank+7,
                    'definition_id':entry['definition_id'],'purchased_rank':rank,'max_rank':row['max_rank']}
            part+=str(rank)
        digits.append(part);offset+=size
    value['sections']['talents']={'config_id':7,'observed_tree_ids':{'10':True,'11':True,'12':True},
        'observed_node_count':len(source['talents']),'nodes':nodes,'staged_changes_observed':False,
        'loadout_export':{'status':'unavailable','reason':'client_not_supported'},
        'definition_allocation':{'coverage':'queried_config_tree_entries','queried_node_count':len(source['talents']),
            'queried_entry_count':len(definitions),'definitions':definitions}}
    return value,'-'.join(digits)


def binary():
    root=Path(__file__).resolve().parents[1]
    return Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
        ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)


class AllocationMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_opt_in_actual_engine_uses_prepared_snapshot_ranks(self):
        if not all(os.environ.get(key) for key in ['AZEROTH_POSITIONAL_ENGINE','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Explicit local engine/public template required; no downloads')
        engine=Path(os.environ['AZEROTH_POSITIONAL_ENGINE']).resolve(strict=True)
        path=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        template=json.loads(path.read_text());player=adapter.single_player(template['raid'])
        source=talent_map()['classes']['hunter'];parts=player['talentsString'].split('-');parts+=['']*(3-len(parts))
        digits=[int(digit) for part,size in zip(parts,source['tree_sizes']) for digit in part.ljust(size,'0')]
        ranks=[min(rank,row['max_rank']) for rank,row in zip(digits,source['talents'])]
        value,_=sample('hunter',ranks)
        with tempfile.TemporaryDirectory() as folder:
            key=SnapshotStore(Path(folder)).ingest(value)
            async with stdio_client(StdioServerParameters(command=str(binary()),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,'prepare_simulator_talents':True})
                    self.assertFalse(reply.isError,reply)
                    prepared=json.loads(reply.content[0].text)['simulator_talent_preparation']
        self.assertEqual(prepared['status'],'prepared_for_snapshot_review')
        _,review,encounter=inputs();review['captured_active_pet']={'present':True,'level':60}
        review['checks']['simulator_equipment_spec']=deepcopy(player['equipment'])
        encounter['simulation_input']['simOptions']={'iterations':200,'randomSeed':'20261109'}
        baseline=adapter.request_from(template,review,encounter)
        altered=deepcopy(template);adapter.single_player(altered['raid'])['talentsString']=prepared['simulator_talent_string']
        request=adapter.request_from(altered,review,encounter)
        first=adapter.run(engine,layout.ENGINE_SHA256,baseline)
        second=adapter.run(engine,layout.ENGINE_SHA256,request)
        first.pop('request_sha256',None);second.pop('request_sha256',None)
        self.assertEqual(first,second)

    async def test_all_classes_exact_mapping_and_no_optimistic_string(self):
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary()),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(value,**options):
                        key=store.ingest(value);before=(Path(folder)/f'{key}.json').read_bytes()
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,**options})
                        self.assertFalse(reply.isError,reply)
                        self.assertEqual((Path(folder)/f'{key}.json').read_bytes(),before)
                        return json.loads(reply.content[0].text)
                    for name in talent_map()['classes']:
                        value,expected=sample(name)
                        data=await call(value,prepare_simulator_talents=True)
                        prepared=data['simulator_talent_preparation']
                        self.assertEqual(prepared['status'],'prepared_for_snapshot_review',name)
                        self.assertEqual(prepared['simulator_talent_string'],expected,name)
                        for flag in ['allocation_match_verified','native_tree_coverage_verified','simulation_model_verified',
                                     'build_legality_verified','native_import_generated','game_execution','live']:
                            self.assertFalse(prepared[flag])
                        self.assertEqual(data['status'],'captured_allocation_for_review')
                        self.assertFalse(data['native_import_available'])
                        self.assertEqual(data['loadout']['nodes'],value['sections']['talents']['nodes'])
                        compared=await call(value,simulator_talent_string=expected)
                        self.assertEqual(compared['simulator_talent_observation']['status'],'observed_ranks_match')
                        old=await call(value)
                        self.assertNotIn('simulator_talent_preparation',old)
                    value,_=sample();cases=[]
                    for staged in [True,None]:
                        changed=deepcopy(value)
                        if staged is None:changed['sections']['talents'].pop('staged_changes_observed')
                        else:changed['sections']['talents']['staged_changes_observed']=staged
                        cases.append((changed,'staged_changes_or_unknown'))
                    missing=deepcopy(value);missing['sections']['talents'].pop('definition_allocation')
                    cases.append((missing,'definition_allocation_not_captured'))
                    missing_class=deepcopy(value);missing_class['sections'].pop('stats')
                    cases.append((missing_class,'captured_class_unmapped'))
                    absent=deepcopy(value);allocation=absent['sections']['talents']['definition_allocation']
                    definition,row=next(iter(allocation['definitions'].items()))
                    allocation['definitions'].pop(definition);allocation['queried_entry_count']-=1
                    absent['sections']['talents']['nodes'].pop(str(row['node_id']),None)
                    cases.append((absent,'pinned_definition_mismatch'))
                    extra=deepcopy(value);allocation=extra['sections']['talents']['definition_allocation']
                    allocation['definitions']['4294967295']={'node_id':4000,'entry_id':4000,'tree_id':10,'purchased_rank':0,'max_rank':1}
                    allocation['queried_entry_count']+=1;allocation['queried_node_count']+=1
                    extra['sections']['talents']['observed_node_count']+=1
                    cases.append((extra,'pinned_definition_mismatch'))
                    limits=deepcopy(value);allocation=limits['sections']['talents']['definition_allocation']
                    definition,row=next(iter(allocation['definitions'].items()));row['max_rank']+=1
                    limits['sections']['talents']['nodes'][str(row['node_id'])]['max_rank']+=1
                    cases.append((limits,'pinned_definition_mismatch'))
                    for changed,reason in cases:
                        prepared=(await call(changed,prepare_simulator_talents=True))['simulator_talent_preparation']
                        self.assertEqual(prepared['status'],'unavailable');self.assertEqual(prepared['reason'],reason)
                        self.assertNotIn('simulator_talent_string',prepared)
                    key=store.ingest(value)
                    for options in [{'prepare_simulator_talents':True,'simulator_talent_string':'1'},
                                    {'prepare_simulator_talents':'PRIVATE'}, {'prepare_simulator_talents':1}]:
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,**options})
                        self.assertTrue(reply.isError);self.assertNotIn('PRIVATE',str(reply))
                    for family in ['retail','classic']:
                        changed=fixture();changed['client']['family']=family;key=store.ingest(changed)
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,'prepare_simulator_talents':True})
                        self.assertTrue(reply.isError)

    async def test_malformed_allocation_groups_rejected_by_both_readers(self):
        base,_=sample();bad=[]
        overfull=deepcopy(base);talents=overfull['sections']['talents'];talents['nodes']={};talents['observed_node_count']=1
        talents['definition_allocation']={'coverage':'queried_config_tree_entries','queried_node_count':1,
            'queried_entry_count':65,'definitions':{str(1000+i):{'node_id':1,'entry_id':2000+i,'tree_id':10,
                'purchased_rank':0,'max_rank':1} for i in range(65)}}
        bad.append(overfull)
        for field,value in [('coverage','all_talents'),('queried_node_count',0),('queried_node_count',True),
            ('queried_entry_count',0),('queried_entry_count',1.0),('definitions',None),('extra','PRIVATE')]:
            changed=deepcopy(base);changed['sections']['talents']['definition_allocation'][field]=value;bad.append(changed)
        for field in ['coverage','queried_node_count','queried_entry_count','definitions']:
            changed=deepcopy(base);changed['sections']['talents']['definition_allocation'].pop(field);bad.append(changed)
        for field,value in [('node_id',0),('entry_id',True),('tree_id',999),('max_rank',None),
            ('purchased_rank',True),('purchased_rank',4294967296),('purchased_rank',6),('extra','PRIVATE')]:
            changed=deepcopy(base)
            next(iter(changed['sections']['talents']['definition_allocation']['definitions'].values()))[field]=value;bad.append(changed)
        for mode in ['definition_key','entry_duplicate','node_tree','active_missing','positive_inactive','tuple_disagrees','null_group','wrong_family']:
            changed=deepcopy(base);talents=changed['sections']['talents'];rows=talents['definition_allocation']['definitions']
            first,second=list(rows)[:2];row=rows[first]
            if mode=='definition_key':rows['01']=rows.pop(first)
            elif mode=='entry_duplicate':rows[second]['entry_id']=row['entry_id']
            elif mode=='node_tree':rows[second].update(node_id=row['node_id'],tree_id=11)
            elif mode=='active_missing':row['entry_id']=4000
            elif mode=='positive_inactive':rows[second]['node_id']=4000
            elif mode=='tuple_disagrees':talents['nodes'][str(row['node_id'])]['purchased_rank']=1
            elif mode=='null_group':talents['definition_allocation']=None
            else:changed['client']['family']='retail'
            bad.append(changed)
        with tempfile.TemporaryDirectory() as folder:
            for index,value in enumerate(bad):
                with self.subTest(index=index),self.assertRaises(ValueError):validate_snapshot(value)
            keys=[]
            for value in bad:
                data=json.dumps(value,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(data).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(data);keys.append(key)
            async with stdio_client(StdioServerParameters(command=str(binary()),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key in keys:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'talents'})
                        self.assertTrue(reply.isError,key);self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(folder,str(reply))


if __name__=='__main__':unittest.main()
