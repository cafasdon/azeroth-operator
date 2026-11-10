"""Profession-slot observations remain separate from affordability and eligibility."""
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


class TrainerProfessionSlotTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_known_blockers_unknowns_and_strict_fields(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            base['sections']['wallet']={'copper':1000}
            base['sections']['trainer_services']={'kind':'tradeskills','entries':{
                '1':{'name':'Profession','status':'available','price_copper':100,'is_profession':True},
                '2':{'name':'Lesson','status':'available','price_copper':200,'is_profession':False},
                '3':{'name':'Unknown','status':'unavailable','price_copper':300},
                '4':{'name':'Known','status':'used'}}}
            keys={}
            for label,full in [('full',True),('open',False),('unknown',None)]:
                value=deepcopy(base)
                if full is not None:value['sections']['trainer_services']['primary_profession_slots_full']=full
                keys[label]=store.ingest(value)
            legacy=deepcopy(base)
            for lesson in legacy['sections']['trainer_services']['entries'].values():lesson.pop('is_profession',None)
            keys['legacy']=store.ingest(legacy)
            pet=deepcopy(base);pet['sections']['trainer_services']={'kind':'pet','entries':{
                '1':{'name':'Pet lesson','status':'available','price_training_points':5},
                '2':{'name':'Advanced pet lesson','status':'unavailable','required_level':22}}}
            pet_key=store.ingest(pet)
            partial_pet=deepcopy(pet)
            partial_pet['sections']['trainer_services']['pet_training_points_available']=6
            partial_pet['sections']['trainer_services']['entries']['3']={'name':'Unpriced lesson','status':'available'}
            partial_pet_id=store.ingest(partial_pet)
            point_cases=[]
            for total,used,trainer_points,pet_level,agree in [(10,3,7,20,True),(3,10,0,19,True),
                                                            (10,3,6,20,False),(-1,3,0,20,None)]:
                capture=deepcopy(pet)
                capture['sections']['trainer_services'].update(pet_level=20,pet_training_points_available=trainer_points)
                capture['sections']['pet_care']={'present':True,'level':pet_level,
                    'training_points':{'total':total,'used':used}}
                point_cases.append((store.ingest(capture),total-used,trainer_points,pet_level,agree))
            bad_values=[]
            for invalid in [None,0,'true']:
                bad=deepcopy(base);bad['sections']['trainer_services']['primary_profession_slots_full']=invalid;bad_values.append(bad)
                bad=deepcopy(base);bad['sections']['trainer_services']['entries']['1']['is_profession']=invalid;bad_values.append(bad)
            bad=deepcopy(base);bad['sections']['trainer_services']['entries']['4']['is_profession']=False;bad_values.append(bad)
            for field in ['primary_profession_slots_full','is_profession']:
                bad=deepcopy(pet)
                if field=='is_profession':bad['sections']['trainer_services']['entries']['1'][field]=False
                else:bad['sections']['trainer_services'][field]=False
                bad_values.append(bad)
            bad_keys=[]
            for value in bad_values:
                with self.assertRaises(ValueError):validate_snapshot(value)
                raw=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def inspect(key,**arguments):
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'trainer_services',
                            'trainer_requirements_review':True,**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    budgets=[]
                    for label,expected in [('full',True),('open',False),('unknown',None),('legacy',None)]:
                        result=await inspect(keys[label],trainer_wallet_projection=True)
                        check=result['trainer_requirements_review']['profession_slot_checks']
                        self.assertEqual(check['primary_profession_slots_full'],expected)
                        self.assertFalse(check['eligibility_verified'])
                        self.assertEqual(set(check['lessons_by_slot']),{'1','2','3'})
                        self.assertEqual(check['lessons_by_slot']['1']['blocked_by_primary_profession_slots'],expected)
                        if label!='legacy':self.assertFalse(check['lessons_by_slot']['2']['blocked_by_primary_profession_slots'])
                        else:self.assertIsNone(check['lessons_by_slot']['2']['blocked_by_primary_profession_slots'])
                        self.assertIsNone(check['lessons_by_slot']['3']['blocked_by_primary_profession_slots'])
                        budgets.append(result['trainer_wallet_projection'])
                    self.assertTrue(all(budget==budgets[0] for budget in budgets))
                    selected=await inspect(keys['full'],trainer_service_slots=[1])
                    self.assertEqual(set(selected['trainer_requirements_review']['profession_slot_checks']['lessons_by_slot']),{'1'})
                    legacy_requirements=(await inspect(pet_key))['trainer_requirements_review']
                    self.assertIsNone(legacy_requirements['profession_slot_checks'])
                    self.assertIsNone(legacy_requirements['unavailable_lessons_by_slot']['2'][
                        'level_shortfall_vs_active_pet_observation'])
                    legacy_points=(await inspect(pet_key,trainer_points_projection=True))['trainer_points_projection']
                    self.assertIsNone(legacy_points['active_pet_capture_context']['spendable_points_observations_agree'])
                    partial=(await inspect(partial_pet_id,trainer_points_projection=True,trainer_balance_reserve=2))[
                        'trainer_points_projection']['after_reserve']
                    self.assertFalse(partial['retained_lessons_affordable'])
                    self.assertIsNone(partial['retained_lessons_shortfall'])
                    self.assertEqual(partial['known_lessons_and_reserve_shortfall'],1)
                    for key,signed,points,level,agree in point_cases:
                        report=await inspect(key,trainer_points_projection=True)
                        projection=report['trainer_points_projection']
                        context=projection['active_pet_capture_context']
                        requirements=report['trainer_requirements_review']
                        self.assertEqual(requirements['active_pet_capture_context'],context)
                        lesson=requirements['unavailable_lessons_by_slot']['2']
                        self.assertEqual(lesson['level_shortfall'],2)
                        self.assertEqual(lesson['level_shortfall_vs_active_pet_observation'],22-level)
                        self.assertEqual(context['active_pet_signed_training_point_balance'],signed)
                        self.assertEqual(context['trainer_training_points_available'],points)
                        self.assertIs(context['levels_agree'],level==20)
                        self.assertIs(context['spendable_points_observations_agree'],agree)
                        self.assertFalse(context['same_pet_verified'])
                        self.assertEqual(projection['training_point_shortfall'],max(0,5-points))
                        reserved=(await inspect(key,trainer_points_projection=True,trainer_balance_reserve=2))[
                            'trainer_points_projection']['after_reserve']
                        expected=({'lesson_slots':['1'],'lesson_count':1,'total_price':5} if points>=7 else
                                  {'lesson_slots':[],'lesson_count':0,'total_price':0} if points>=2 else None)
                        self.assertEqual(reserved['cheapest_affordable_known_lessons'],expected)
                        self.assertEqual(reserved['individual_lesson_affordability']['1'],
                                         {'affordable':points>=7,'shortfall':max(0,7-points)})
                        self.assertEqual(reserved['retained_lessons_shortfall'],max(0,7-points))
                        self.assertEqual(reserved['known_lessons_and_reserve_shortfall'],max(0,7-points))
                    for key in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'trainer_services',
                                                                          'trainer_requirements_review':True})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
