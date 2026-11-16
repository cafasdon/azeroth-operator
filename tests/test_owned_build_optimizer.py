"""Real saved ownership, generation and native joint windows over public fixtures."""
import asyncio
from copy import deepcopy
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from shared.snapshots import SnapshotStore
from tools.account_inventory import AccountInventory
from tools.owned_build_source import OwnedBuildSource
from tools.forever_talent_identity import talent_map
from tools import joint_build_optimizer as joint
from tests.benchmark_joint_build_optimizer import fixture

ROOT=Path(__file__).resolve().parents[1]
RUNTIME=ROOT/'local-data/forever-build-runtime'/joint.layout.ENGINE_SHA256


def public_capture(model, name='SyntheticMain', minutes=60):
    player=model['raid']['parties'][0]['players'][0]
    tree=talent_map()['classes']['hunter']; definitions={}; nodes={}; index=0
    parts=player['talentsString'].split('-');parts+=['']*(3-len(parts))
    for tree_index,(part,size) in enumerate(zip(parts,tree['tree_sizes']),1):
        for digit in part.ljust(size,'0'):
            row=tree['talents'][index]; node=1000+index; entry=10000+index
            allocation=dict(node_id=node,entry_id=entry,tree_id=tree_index,purchased_rank=int(digit),max_rank=row['max_rank'])
            definitions[str(row['definition_id'])]=allocation
            if int(digit):
                nodes[str(node)]=dict(entry_id=entry,tree_id=tree_index,rank=int(digit),definition_id=row['definition_id'],
                                      purchased_rank=int(digit),max_rank=row['max_rank'])
            index+=1
    equipment={}
    for slot,item in zip([1,2,3,15,5,9,10,6,7,8,11,12,13,14,16,17,18],player['equipment']['items']):
        if item:
            equipment[str(slot)]=dict(item_id=item['id'],
                link=f"item:{item['id']}:{item.get('enchant',0)}:0:0:0:0:{item.get('randomSuffix',0)}:0")
    return dict(schema_version=1,source='addon',captured_at=(datetime.now(timezone.utc)-timedelta(minutes=minutes)).replace(microsecond=0).isoformat().replace('+00:00','Z'),
        client=dict(family='forever',variant='live',build='70862',interface=16001),character=dict(name=name,realm='Offline'),
        sections=dict(stats=dict(level=60,**{'class':'HUNTER','race':'Orc','client_version':'synthetic'}),equipment=equipment,bags={'0':{}},
            professions={str(i):dict(skill_line=skill,name='Synthetic profession',rank=300,max_rank=300)
                         for i,skill in enumerate((202,164),1)},pet_care=dict(present=True,level=60),
            talents=dict(config_id=7,observed_tree_ids={str(i):True for i in range(1,4)},observed_node_count=index,
                staged_changes_observed=False,nodes=nodes,definition_allocation=dict(coverage='queried_config_tree_entries',queried_node_count=index,
                    queried_entry_count=index,definitions=definitions))))


class OwnedBuildOptimizerTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        from tools.owned_build_optimizer import OwnedBuildOptimizer
        self.temp=tempfile.TemporaryDirectory(dir=ROOT/'local-data');self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.store=SnapshotStore(self.root/'snapshots')
        self.database=self.root/'inventory.sqlite';self.inventory=AccountInventory(self.database)
        self.export=self.root/'AzerothOperator.lua';self.model=fixture(20261116,2)['base_request']
        # The upstream preset's wrist enchant does not fit the captured catalogue
        # projection. This saved-data fixture declares an unenchanted wrist.
        self.model['raid']['parties'][0]['players'][0]['equipment']['items'][5].pop('enchant',None)
        self.value=public_capture(self.model);self.identifier=self.ingest(self.value);self.save_export(self.value)
        self.config=dict(reader=str(ROOT/'target/debug/azeroth-operator.exe'),snapshots=str(self.store.root),active_export=str(self.export),
            inventory_database=str(self.database),account_key='ownerA',client=deepcopy(self.value['client']))
        self.source=OwnedBuildSource(self.config)
        self.optimizer=OwnedBuildOptimizer(self.source,ROOT/'local-data/forever/catalogue.json',RUNTIME/'equipment-probe.exe',RUNTIME/'wowsimcli-positional.exe')

    def ingest(self,value,account='ownerA'):
        key=self.store.ingest(value);self.inventory.ingest(self.store.root,account,key);return key

    def save_export(self,value):
        self.export.write_bytes(('AzerothOperatorExport = "'+json.dumps(value,separators=(',',':')).encode().hex()+'"\nAzerothOperatorOperations = nil\n').encode())

    def assert_unready(self, report):
        self.assertEqual(report['status'],'plan_only');self.assertTrue(report['blocked_reasons'])
        self.assertFalse(report['simulation_performed']);self.assertFalse(report['active_character_verified'])

    async def test_unsupported_real_level_shape_is_bound_without_engine(self):
        self.value['sections']['stats']['level']=22
        self.value['captured_at']=(datetime.fromisoformat(self.value['captured_at'].replace('Z','+00:00'))+timedelta(seconds=1)).isoformat().replace('+00:00','Z')
        self.identifier=self.ingest(self.value);self.save_export(self.value)
        self.optimizer.engine=self.root/'must-not-execute.exe'
        report=await self.optimizer.prepare(model=self.model)
        self.assert_unready(report);self.assertIn('unsupported_character_level',report['blocked_reasons'])
        self.assertEqual(report['snapshot_id'],self.identifier)

    async def test_missing_model_does_not_substitute_public_hunter_case(self):
        report=await self.optimizer.prepare()
        self.assert_unready(report)
        self.assertEqual(report['snapshot_id'],self.identifier)

    async def test_owned_generated_256_case_and_real_native_window(self):
        equipment=self.model['raid']['parties'][0]['players'][0]['equipment']
        choices=[dict(slot=i,items=[equipment['items'][i],{}]) for i in range(7)]
        talents=[dict(definition_id=134422,ranks=[4,5]),dict(definition_id=134438,ranks=[3,4])]
        prepared=await self.optimizer.prepare(model=self.model,gear_choices=choices,talent_choices=talents,limit=16)
        self.assertEqual(prepared['status'],'ready');self.assertEqual(prepared['total_decimal'],'256')
        self.assertEqual(prepared['snapshot_id'],self.identifier)
        self.assertTrue(prepared['snapshot_reference_verified']);self.assertFalse(prepared['all_owned_items_covered'])
        result=await self.optimizer.evaluate(prepared['search_id'],offset='127',limit=2)
        self.assertEqual(result['evaluated_coordinates'],2)
        self.assertTrue(all(row['mean_dps']>0 for row in result['results']))
        self.assertTrue(result['snapshot_reference_verified']);self.assertFalse(result['global_optimum_verified'])
        self.assertNotIn('SyntheticMain',json.dumps(result['results']))
        self.assertFalse(result['active_character_verified'])

    async def test_unowned_known_item_and_missing_modifier_cannot_enter_pool(self):
        for item in [dict(id=4549),dict(id=16677,enchant=999999)]:
            with self.subTest(item=item):
                report=await self.optimizer.prepare(model=self.model,gear_choices=[dict(slot=10,items=[item])])
                self.assert_unready(report)

    async def test_default_is_invalidated_by_character_switch_explicit_is_stable(self):
        selected=await self.optimizer.prepare(model=self.model,gear_choices=[])
        self.assertEqual(selected['status'],'ready')
        alt=public_capture(self.model,'SyntheticAlt',minutes=5);self.ingest(alt);self.save_export(alt)
        with self.assertRaises(ValueError):await self.optimizer.evaluate(selected['search_id'],limit=1)
        explicit=dict(name='SyntheticMain',realm='Offline')
        saved=await self.optimizer.prepare(character=explicit,model=self.model,gear_choices=[])
        self.assertEqual(saved['status'],'ready')
        with self.assertRaises(ValueError):await self.optimizer.evaluate(saved['search_id'],limit=1)
        result=await self.optimizer.evaluate(saved['search_id'],character=explicit,limit=1)
        self.assertEqual(result['evaluated_coordinates'],1)

    async def test_incomplete_talents_and_conflicting_template_stay_unready(self):
        self.value['sections']['talents'].pop('definition_allocation')
        self.value['captured_at']=(datetime.fromisoformat(self.value['captured_at'].replace('Z','+00:00'))+timedelta(seconds=1)).isoformat().replace('+00:00','Z')
        self.identifier=self.ingest(self.value);self.save_export(self.value)
        self.assert_unready(await self.optimizer.prepare(model=self.model,gear_choices=[]))

    async def test_data_and_model_are_unchanged_and_paths_not_in_report(self):
        before={p.relative_to(self.root):hashlib.sha256(p.read_bytes()).hexdigest() for p in self.root.rglob('*') if p.is_file()}
        model=deepcopy(self.model);report=await self.optimizer.prepare(model=self.model,gear_choices=[])
        self.assertEqual(self.model,model)
        after={p.relative_to(self.root):hashlib.sha256(p.read_bytes()).hexdigest() for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before,after);self.assertNotIn(str(self.root),json.dumps(report))

    async def test_native_numeric_race_enum_matches_named_race(self):
        numeric=deepcopy(self.model);numeric['raid']['parties'][0]['players'][0]['race']=6;numeric['type']=1
        named=await self.optimizer.prepare(model=self.model,gear_choices=[])
        result=await self.optimizer.prepare(model=numeric,gear_choices=[])
        self.assertEqual(result['status'],'ready');self.assertEqual(result['search_id'],named['search_id'])

    async def test_complete_zero_allocation_preserves_empty_talent_string(self):
        self.model['raid']['parties'][0]['players'][0]['talentsString']=''
        self.value=public_capture(self.model,minutes=50)
        self.identifier=self.ingest(self.value);self.save_export(self.value)
        report=await self.optimizer.prepare(model=self.model,gear_choices=[])
        self.assertEqual(report['status'],'ready')

    async def test_shared_bank_stock_is_excluded_from_owned_pool(self):
        self.value['captured_at']=(datetime.fromisoformat(self.value['captured_at'].replace('Z','+00:00'))+timedelta(seconds=1)).isoformat().replace('+00:00','Z')
        self.value['sections']['account_bank']={'13':{'1':dict(item_id=4549,count=2,link='item:4549:0:0:0:0:0:0')}}
        self.identifier=self.ingest(self.value);self.save_export(self.value)
        report=await self.optimizer.prepare(model=self.model,gear_choices=[dict(slot=10,items=[dict(id=4549)])])
        self.assert_unready(report);self.assertIn('account_bank',report['excluded_storage'])
        self.assertEqual(report['resolved_physical_count'],len(self.value['sections']['equipment']))

    async def test_default_domains_and_wrong_model_identity(self):
        report=await self.optimizer.prepare(model=self.model)
        self.assertEqual(report['status'],'ready')
        bad=deepcopy(self.model);bad['raid']['parties'][0]['players'][0]['race']='RaceHuman'
        with self.assertRaises(ValueError):await self.optimizer.prepare(model=bad,gear_choices=[])

    async def test_unresolved_equipped_variant_receives_no_score(self):
        self.value['sections']['equipment']['1']['link']='item:16677:999999:0:0:0:0:0:0'
        self.value['captured_at']=(datetime.fromisoformat(self.value['captured_at'].replace('Z','+00:00'))+timedelta(seconds=1)).isoformat().replace('+00:00','Z')
        self.identifier=self.ingest(self.value);self.save_export(self.value)
        report=await self.optimizer.prepare(model=self.model,gear_choices=[])
        self.assert_unready(report);self.assertIn('unresolved_captured_equipment',report['blocked_reasons'])


if __name__=='__main__':unittest.main()
