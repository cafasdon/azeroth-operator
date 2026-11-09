"""Queried inactive zero ranks must reach simulator preflight and MCP comparison."""
from copy import deepcopy
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_talent_allocation import sample, binary
from test_simulator_level import inputs
from tools.forever_talent_identity import ConflictingTalentObservation, check_talent_identity, talent_map
from tools.run_forever_sim import request_from
from tools import run_forever_sim as adapter
from tools import positional_equipment as layout


def conflicting_string(value,text,name='hunter'):
    allocation=value['sections']['talents']['definition_allocation']['definitions']
    inactive=next(identifier for identifier,row in allocation.items()
                  if row['purchased_rank']==0 and str(row['node_id']) not in value['sections']['talents']['nodes'])
    index=next(index for index,row in enumerate(talent_map()['classes'][name]['talents'])
               if str(row['definition_id'])==inactive)
    digits=list(text.replace('-',''));digits[index]='1';sizes=talent_map()['classes'][name]['tree_sizes']
    start=0;parts=[]
    for size in sizes:parts.append(''.join(digits[start:start+size]));start+=size
    return '-'.join(parts),int(inactive)


class AllocationSimulationTests(unittest.IsolatedAsyncioTestCase):
    async def test_opt_in_matching_allocation_keeps_real_engine_request(self):
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
        value,_=sample('hunter',[min(rank,row['max_rank']) for rank,row in zip(digits,source['talents'])])
        value['sections']['equipment']={'16':{'item_id':123}}
        value['sections']['pet_care']={'present':True,'level':60}
        with tempfile.TemporaryDirectory() as folder:
            key=SnapshotStore(Path(folder)).ingest(value)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary()),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                        'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                    self.assertFalse(reply.isError,reply);review=json.loads(reply.content[0].text)
        # Public template gear is an independent known engine input, not an owned-gear qualification.
        review['checks']['simulator_equipment_spec']=deepcopy(player['equipment'])
        _,_,encounter=inputs();encounter['simulation_input']['simOptions']={'iterations':200,'randomSeed':'20261109'}
        checked=request_from(template,review,encounter)
        legacy=deepcopy(review);legacy.pop('captured_talent_observation')
        baseline=request_from(template,legacy,encounter)
        self.assertEqual(checked,baseline)
        self.assertEqual(adapter.run(engine,layout.ENGINE_SHA256,checked),adapter.run(engine,layout.ENGINE_SHA256,baseline))

    async def test_actual_equipment_review_blocks_inactive_purchase_before_engine(self):
        value,text=sample();value['sections']['equipment']={'16':{'item_id':123}}
        wrong,definition=conflicting_string(value,text)
        with tempfile.TemporaryDirectory() as folder:
            key=SnapshotStore(Path(folder)).ingest(value)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary()),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                        'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                    self.assertFalse(reply.isError,reply);review=json.loads(reply.content[0].text)
                    self.assertEqual(review['captured_talent_observation']['definition_allocation'],
                        value['sections']['talents']['definition_allocation'])
                    template,fixture_review,encounter=inputs()
                    # Equipment is an independent synthetic adapter fixture; this tests talent preflight.
                    review['checks']['simulator_equipment_spec']=fixture_review['checks']['simulator_equipment_spec']
                    template['raid']['parties'][0]['players'][0]['talentsString']=text
                    self.assertIn('raid',request_from(template,review,encounter))
                    template['raid']['parties'][0]['players'][0]['talentsString']=wrong
                    with self.assertRaises(ConflictingTalentObservation):request_from(template,review,encounter)
                    for name,data in [('template',template),('review',review),('encounter',encounter)]:
                        (Path(folder)/f'{name}.json').write_text(json.dumps(data))
                    cli=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                        '--engine',str(Path(folder)/'PRIVATE-never-run.exe'),'--engine-sha256','a'*64,
                        '--template',str(Path(folder)/'template.json'),'--equipment-review',str(Path(folder)/'review.json'),
                        '--encounter-lookup',str(Path(folder)/'encounter.json')],capture_output=True,text=True,timeout=10)
                    self.assertEqual(cli.returncode,1);self.assertEqual(cli.stdout,'')
                    self.assertIn('purchased ranks',cli.stderr);self.assertNotIn('PRIVATE',cli.stderr);self.assertNotIn(folder,cli.stderr)
                    reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,'simulator_talent_string':wrong})
                    self.assertFalse(reply.isError,reply)
                    report=json.loads(reply.content[0].text)['simulator_talent_observation']
                    self.assertEqual(report['status'],'mismatch')
                    self.assertIn(definition,[row['definition_id'] for row in report['conflicts']])

    async def test_all_class_reports_match_and_absence_is_not_zero(self):
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            async with stdio_client(StdioServerParameters(command=str(binary()),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def compare(value,text):
                        key=store.ingest(value);before=(Path(folder)/f'{key}.json').read_bytes()
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':key,'simulator_talent_string':text})
                        self.assertFalse(reply.isError,reply)
                        report=json.loads(reply.content[0].text)['simulator_talent_observation']
                        observed={field:value['sections']['talents'][field] for field in ['config_id','nodes',
                            'observed_tree_ids','observed_node_count','definition_allocation','staged_changes_observed']
                            if field in value['sections']['talents']}
                        self.assertEqual(report,check_talent_identity({'talentsString':text},observed,
                            value['sections']['stats']['class'].lower(),False))
                        self.assertEqual((Path(folder)/f'{key}.json').read_bytes(),before)
                        return report,observed
                    for name in talent_map()['classes']:
                        value,text=sample(name);wrong,definition=conflicting_string(value,text,name)
                        report,_=await compare(value,text)
                        self.assertEqual(report['basis'],'captured_queried_definition_allocations_and_pinned_definitions')
                        self.assertEqual(report['compared_definition_count'],len(talent_map()['classes'][name]['talents']))
                        self.assertEqual(report['missing_pinned_definition_count'],0)
                        report,observed=await compare(value,wrong)
                        self.assertEqual(report['status'],'mismatch')
                        self.assertEqual(report['conflicts'],[{'definition_id':definition,'captured_purchased_rank':0,'template_effective_rank':1}])
                        with self.assertRaises(ConflictingTalentObservation):
                            check_talent_identity({'talentsString':wrong},observed,name)
                        for stage in [True,None]:
                            changed=deepcopy(value)
                            if stage is None:changed['sections']['talents'].pop('staged_changes_observed')
                            else:changed['sections']['talents']['staged_changes_observed']=stage
                            report,_=await compare(changed,wrong)
                            self.assertEqual(report['status'],'staged_changes' if stage else 'unverified')
                    value,text=sample()
                    extra=deepcopy(value);talents=extra['sections']['talents'];allocation=talents['definition_allocation']
                    for index in range(2):
                        allocation['definitions'][str(4294967294+index)]={'node_id':4000,'entry_id':4000+index,
                            'tree_id':10,'purchased_rank':0,'max_rank':1}
                    talents['observed_node_count']+=1;allocation['queried_node_count']+=1;allocation['queried_entry_count']+=2
                    report,_=await compare(extra,text)
                    self.assertEqual(report['unknown_node_count'],1);self.assertEqual(report['unmapped_definition_count'],2)
                    wrong,definition=conflicting_string(value,text)
                    row=value['sections']['talents']['definition_allocation']['definitions'].pop(str(definition))
                    value['sections']['talents']['definition_allocation']['queried_entry_count']-=1
                    report,observed=await compare(value,wrong)
                    self.assertEqual(report['missing_pinned_definition_count'],1)
                    self.assertEqual(report['status'],'observed_ranks_match')
                    self.assertFalse(report['allocation_match_verified'])
                    check_talent_identity({'talentsString':wrong},observed,'hunter')


class StandaloneAllocationTests(unittest.TestCase):
    def test_standalone_export_contract_rejects_tampering(self):
        value,text=sample();talents=value['sections']['talents']
        observed={field:talents[field] for field in ['config_id','nodes','observed_tree_ids','observed_node_count',
            'staged_changes_observed','definition_allocation']}
        mutations=[lambda o:o.update(definition_allocation=None),
            lambda o:o['definition_allocation'].update(queried_entry_count=0),
            lambda o:o['definition_allocation'].update(queried_node_count=True),
            lambda o:o['definition_allocation'].update(private='PRIVATE'),
            lambda o:next(iter(o['definition_allocation']['definitions'].values())).update(entry_id=True),
            lambda o:next(iter(o['definition_allocation']['definitions'].values())).update(node_id=4000),
            lambda o:next(iter(o['definition_allocation']['definitions'].values())).update(tree_id=999),
            lambda o:next(iter(o['definition_allocation']['definitions'].values())).update(purchased_rank=1),
            lambda o:next(iter(o['definition_allocation']['definitions'].values())).update(max_rank=0)]
        for mutation in mutations:
            changed=deepcopy(observed);mutation(changed)
            with self.assertRaises(ValueError):check_talent_identity({'talentsString':text},changed,'hunter')


if __name__=='__main__':unittest.main()
