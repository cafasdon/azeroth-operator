"""Known native category contradictions cannot qualify a simulated equipment set."""
from copy import deepcopy
from contextlib import redirect_stdout, redirect_stderr
import io
import json
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.rank_forever_slot import collect, main as rank_main
from tools.run_forever_sim import request_from, ConflictingNativeSlotCategory


class NativeSlotConflictTests(unittest.IsolatedAsyncioTestCase):
    def test_direct_adapter_rejects_conflicts_even_with_copied_spec(self):
        template,review,encounter=inputs()
        original=deepcopy((template,review,encounter))
        self.assertIn('raid',request_from(template,review,encounter))
        bad=deepcopy(review);bad['checks']['native_slot_category_conflicts']=[{'slot':11,'item_id':4549}]
        with self.assertRaisesRegex(ConflictingNativeSlotCategory,'Native slot-category conflicts'):
            request_from(template,bad,encounter)
        self.assertEqual((template,review,encounter),original)

    def test_legacy_annotations_and_metadata_cannot_bypass_guard(self):
        template,review,encounter=inputs()
        for change in [
            {'target_slot':11,'native_slot_category':{'target_slot_category_matches':False}},
            {'target_slot':11,'selected_item_observed':{'item_id':4549,'equip_location':'INVTYPE_HEAD'}},
            {'target_slot':11,'native_slot_category':{'target_slot_category_matches':True},
             'selected_item_observed':{'item_id':4549,'equip_location':'INVTYPE_HEAD'}},
        ]:
            bad=deepcopy(review);bad['checks']['simulator_equipment_spec']['items'][10]={'id':4549};bad['changes']=[change]
            with self.assertRaises(ConflictingNativeSlotCategory):request_from(template,bad,encounter)
        for target,location,index in [(17,'INVTYPE_2HWEAPON',15),(17,'INVTYPE_WEAPON',15),
                (11,'',10),(11,'INVTYPE_FUTURE',10),(11,None,10)]:
            good=deepcopy(review);good['checks']['simulator_equipment_spec']['items'][index]={'id':4549}
            good['changes']=[{'target_slot':target,'selected_item_observed':{'equip_location':location}}]
            self.assertIn('raid',request_from(template,good,encounter))
        # False observations on non-simulated/cleared slots do not describe a projected item.
        for target in [4,19,17]:
            cleared=deepcopy(review);cleared['changes']=[{'target_slot':target,
                'native_slot_category':{'target_slot_category_matches':False},'selected_item_observed':{'equip_location':'INVTYPE_HEAD'}}]
            self.assertIn('raid',request_from(template,cleared,encounter))

    def test_clis_reject_before_engine_and_mask_private_details(self):
        template,review,encounter=inputs()
        review['checks']['native_slot_category_conflicts']=[{'slot':11,'private':'PRIVATE'}]
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);binary=root/'mcp.exe';binary.touch()
            for name,data in [('template',template),('review',review),('encounter',encounter)]:
                (root/f'{name}.json').write_text(json.dumps(data))
            result=subprocess.run([sys.executable,'-m','tools.run_forever_sim','--engine',str(root/'must-not-run.exe'),
                '--engine-sha256','a'*64,'--template',str(root/'template.json'),'--equipment-review',str(root/'review.json'),
                '--encounter-lookup',str(root/'encounter.json')],capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,1);self.assertEqual(result.stdout,'')
            self.assertIn('Native slot-category conflicts',result.stderr)
            self.assertNotIn('PRIVATE',result.stderr);self.assertNotIn(str(root),result.stderr)
            command=['rank','--mcp',str(binary),'--snapshot-id','fixture','--slot','finger1',
                '--template',str(root/'template.json'),'--plan-only']
            out=io.StringIO();err=io.StringIO()
            with patch.object(sys,'argv',command),patch('tools.rank_forever_slot.collect',side_effect=
                    ExceptionGroup('wrapped',[ConflictingNativeSlotCategory('Captured baseline has native slot-category conflicts')])),\
                    patch('tools.rank_forever_slot.rank_encounter') as run,redirect_stdout(out),redirect_stderr(err),self.assertRaises(SystemExit):
                rank_main()
            self.assertEqual(out.getvalue(),'');self.assertIn('baseline has native slot-category conflicts',err.getvalue());run.assert_not_called()

    async def test_projected_baseline_candidates_and_ranker(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE',root/'local-data/forever/catalogue.json'))
        if not catalogue.is_file():self.skipTest('Requires pinned local catalogue; no download')
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['stats']={'level':60,'class':'HUNTER','race':'Human','client_version':'1.60.1'}
        ring={'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}
        selection={'target_slot':11,'expected_item_id':4549,'source':{'kind':'bag','bag':0,'slot':1}}
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys={}
            for location in ['INVTYPE_HEAD','INVTYPE_FINGER','','INVTYPE_FUTURE',None]:
                value=deepcopy(base);item=deepcopy(ring)
                if location is not None:item['equip_location']=location
                value['sections']['bags']={'0':{'1':item}};keys[location]=store.ingest(value)
            conflict=deepcopy(base);equipped=deepcopy(ring);equipped.pop('count');equipped['equip_location']='INVTYPE_HEAD'
            conflict['sections']['equipment']={'11':equipped};baseline_key=store.ingest(conflict)
            clearing=deepcopy(base);clearing['sections']['equipment']={'17':deepcopy(equipped),'4':deepcopy(equipped)}
            clearing['sections']['bags']={'0':{'1':{'item_id':2042,'count':1,'link':'item:2042:0:0:0:0:0:0','equip_location':'INVTYPE_2HWEAPON'}}}
            clear_key=store.ingest(clearing)
            env={**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    for location,key in keys.items():
                        review=await call('review_equipment_selection',snapshot_id=key,selections=[selection])
                        checks=review['checks'];conflicts=checks['native_slot_category_conflicts']
                        if location=='INVTYPE_HEAD':
                            self.assertIsNone(checks['simulator_equipment_spec']);self.assertEqual([r['slot'] for r in conflicts],[11])
                            self.assertEqual(conflicts[0]['observed_equip_location'],'INVTYPE_HEAD')
                        else:self.assertIsNotNone(checks['simulator_equipment_spec']);self.assertEqual(conflicts,[])
                    row=(await call('review_forever_owned_gear',snapshot_id=keys['INVTYPE_HEAD'],simulator_slot='finger1'))['items'][0]
                    self.assertEqual(row['native_slot_category_conflict']['slot'],11)
                    for selections,expected in [([],[11]),([{'target_slot':11,'source':{'kind':'equipment','slot':11},'expected_item_id':4549}],[11]),
                            ([{'target_slot':12,'source':{'kind':'equipment','slot':11},'expected_item_id':4549}],[12])]:
                        review=await call('review_equipment_selection',snapshot_id=baseline_key,selections=selections)
                        self.assertIsNone(review['checks']['simulator_equipment_spec'])
                        self.assertEqual([r['slot'] for r in review['checks']['native_slot_category_conflicts']],expected)
                    cleared=await call('review_equipment_selection',snapshot_id=clear_key,selections=[{
                        'target_slot':16,'source':{'kind':'bag','bag':0,'slot':1},'expected_item_id':2042}],allow_two_hand_off_hand_clear=True)
                    self.assertEqual(cleared['checks']['native_slot_category_conflicts'],[])
                    self.assertEqual(cleared['checks']['simulator_cleared_equipment_slots'],[17])
                    self.assertIsNotNone(cleared['checks']['simulator_equipment_spec'])
                    # Cross-check the legacy Python guard against the actual Rust category caller.
                    from tools.run_forever_sim import NATIVE_CATEGORY_SLOTS, PROJECTED_SLOT_INDEX
                    template,skeleton,encounter=inputs()
                    for token in NATIVE_CATEGORY_SLOTS:
                        sample=deepcopy(base);item=deepcopy(ring);item['equip_location']='INVTYPE_'+token
                        sample['sections']['bags']={'0':{'1':item}};key=store.ingest(sample)
                        for target,index in PROJECTED_SLOT_INDEX.items():
                            report=await call('review_equipment_selection',snapshot_id=key,selections=[{**selection,'target_slot':target}])
                            category=report['changes'][0]['native_slot_category']['target_slot_category_matches']
                            legacy=deepcopy(skeleton);legacy['changes']=report['changes']
                            legacy['changes'][0].pop('native_slot_category')
                            legacy['checks']['simulator_equipment_spec']['items'][index]={'id':4549}
                            if category is False:
                                with self.assertRaises(ConflictingNativeSlotCategory):request_from(template,legacy,encounter)
                            else:self.assertIn('raid',request_from(template,legacy,encounter))
            with tempfile.TemporaryFile(mode='w+') as errlog,patch.dict(os.environ,env):
                for slots in [['finger1'],['finger1','finger2']]:
                    drafts,skipped,*_=await collect(binary,keys['INVTYPE_HEAD'],slots,errlog)
                    self.assertEqual(len(drafts),1)
                    self.assertTrue(skipped)
                    self.assertTrue(all(r['reason']=='native_slot_category_conflict' for r in skipped))
                with self.assertRaises(ExceptionGroup) as rejected:
                    await collect(binary,baseline_key,['finger1'],errlog)
                def leaves(error):
                    return [leaf for child in error.exceptions for leaf in leaves(child)] if isinstance(error,BaseExceptionGroup) else [error]
                self.assertTrue(any(isinstance(error,ValueError) and 'baseline has native slot-category conflicts' in str(error)
                                    for error in leaves(rejected.exception)))
                drafts,skipped,*_=await collect(binary,keys[None],['finger1'],errlog)
                self.assertEqual(len(drafts),2);self.assertEqual(skipped,[])


if __name__=='__main__':unittest.main()
