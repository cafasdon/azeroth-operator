"""Gear products, declared physical copies and actual offline entry points."""
import asyncio
from copy import deepcopy
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools import gear_builds as gear

ROOT = Path(__file__).resolve().parents[1]
CATALOGUE = ROOT/'local-data/forever/catalogue.json'
PROBE = Path(os.environ.get('AZEROTH_EQUIPMENT_PROBE','C:/Temp/codex/forever-positional-v4/artifacts/equipment-probe.exe'))
PATHS = dict(catalogue_path=CATALOGUE,probe_path=PROBE)


def context(**changes):
    return dict(class_id=1,specialization='dpsWarrior',race='RaceOrc',level=60,professions=[])|changes


def equipment(**slots):
    items = [{} for _ in range(17)]
    for slot,item in slots.items():
        items[int(slot)] = item
    return dict(items=items,preserveSlots=True)


def inventory(*items,count=1):
    return [dict(item=item,count=count) for item in items]


def request():
    return dict(context=context(),base_equipment=equipment(),inventory=inventory(dict(id=4549),dict(id=4550)),
                choices=[dict(slot=10,items=[{},dict(id=4549),dict(id=4550)]),dict(slot=11,items=[{},dict(id=4549),dict(id=4550)])])


class GearBuildTests(unittest.TestCase):
    def codes(self,context_value,value,owned):
        return {v['code'] for v in gear.validate_gear_build(context_value,value,owned,**PATHS)['violations']}

    def test_independent_product_and_physical_copies(self):
        value = request(); original = deepcopy(value); result = gear.generate_gear_builds(**value,**PATHS)
        expected = [(a,b) for a,b in itertools.product((0,4549,4550),repeat=2) if not a or a!=b]
        actual = []
        for row in result['accepted']:
            entry = result['references'][row['reference_sha256']]
            self.assertEqual(entry['category'],'gear'); self.assertEqual(gear.wire.digest(entry),row['reference_sha256'])
            actual.append(tuple(entry['value']['items'][s].get('id',0) for s in (10,11)))
        self.assertEqual(actual,expected); self.assertEqual(value,original)
        self.assertEqual(result['examined_count'],9); self.assertEqual(result['accepted_count'],7)
        self.assertEqual(result['rejection_reasons'],dict(declared_copies_exceeded=2))
        self.assertTrue(result['declared_matrix_complete']); self.assertFalse(result['ownership_verified'])
        self.assertFalse(result['native_legality_verified']); self.assertFalse(result['full_gear_space_covered'])
        result = gear.validate_gear_build(context(),equipment(**{'10':dict(id=4549),'11':dict(id=4549)}),inventory(dict(id=4549),count=2),**PATHS)
        self.assertTrue(result['valid'])

    def test_unique_categories_slots_two_hands_and_dual_wield(self):
        unique = dict(id=1076)
        self.assertIn('unique_item',self.codes(context(),equipment(**{'10':unique,'11':unique}),inventory(unique,count=2)))
        self.assertIn('limit_category',self.codes(context(),equipment(**{'12':dict(id=18706),'13':dict(id=19024)}),inventory(dict(id=18706),dict(id=19024))))
        self.assertIn('slot_fit',self.codes(context(),equipment(**{'0':dict(id=4549)}),inventory(dict(id=4549))))
        self.assertIn('two_hand_conflict',self.codes(context(),equipment(**{'14':dict(id=791),'15':dict(id=754)}),inventory(dict(id=791),dict(id=754))))
        self.assertIn('slot_fit',self.codes(context(),equipment(**{'15':dict(id=791)}),inventory(dict(id=791))))
        paladin = context(class_id=2,specialization='holyPaladin',race='RaceHuman')
        self.assertIn('dual_wield',self.codes(paladin,equipment(**{'15':dict(id=754)}),inventory(dict(id=754))))
        self.assertNotIn('dual_wield',self.codes(context(),equipment(**{'15':dict(id=754)}),inventory(dict(id=754))))

    def test_actor_class_faction_profession_and_unknown_constraints(self):
        self.assertIn('faction_restriction',self.codes(context(),equipment(**{'7':dict(id=20041)}),inventory(dict(id=20041))))
        self.assertIn('class_restriction',self.codes(context(),equipment(**{'15':dict(id=6898)}),inventory(dict(id=6898))))
        mage = context(class_id=8,specialization='mage',race='RaceOrc')
        self.assertIn('profession_required',self.codes(mage,equipment(**{'4':dict(id=14152)}),inventory(dict(id=14152))))
        mage['professions']=['Tailoring']
        self.assertTrue(gear.validate_gear_build(mage,equipment(**{'4':dict(id=14152)}),inventory(dict(id=14152)),**PATHS)['valid'])
        self.assertIn('unknown_item',self.codes(context(),equipment(**{'10':dict(id=2147483647)}),[]))
        for change in (dict(level=59),dict(class_id=True),dict(specialization='mage'),dict(race='RaceBloodElf'),dict(professions=['Unknown']),dict(race=6),dict(professions=[4]),dict(professions=['Engineering','Blacksmithing','Tailoring'])):
            with self.subTest(change=change),self.assertRaises(ValueError):
                gear.validate_gear_build(context(**change),equipment(),[],**PATHS)
        missing = context(); missing.pop('race')
        with self.assertRaises(ValueError): gear.validate_gear_build(missing,equipment(),[],**PATHS)

    def test_modifiers_fail_closed_and_exact_variants(self):
        altered = dict(id=866,randomSuffix=12718)
        self.assertIn('declared_copies_exceeded',self.codes(context(),equipment(**{'14':altered}),inventory(dict(id=866))))
        invalid = dict(id=866,randomSuffix=1)
        self.assertIn('suffix_not_available',self.codes(context(),equipment(**{'14':invalid}),inventory(invalid)))
        invalid = dict(id=720,enchant=2147483647)
        self.assertIn('unknown_enchant',self.codes(context(),equipment(**{'6':invalid}),inventory(invalid)))
        with patch.object(gear,'PROBE_SHA256','0'*64):
            result = gear.generate_gear_builds(context(),equipment(**{'14':altered}),inventory(altered),[],**PATHS)
        self.assertEqual(result['unresolved_count'],1); self.assertEqual(result['accepted_count'],0)
        self.assertFalse(result['declared_matrix_complete']); self.assertIsNone(result['axis'])

    def test_windows_resume_drift_and_strict_shapes(self):
        full = gear.generate_gear_builds(**request(),**PATHS)
        first = gear.generate_gear_builds(**request(),limit=3,**PATHS)
        second = gear.generate_gear_builds(**request(),offset=first['next_offset'],expected_search_id=first['search_id'],**PATHS)
        self.assertEqual(first['accepted']+second['accepted'],full['accepted']); self.assertFalse(second['declared_matrix_complete'])
        drift = request(); drift['inventory'][0]['count']=2
        with self.assertRaises(ValueError): gear.generate_gear_builds(**drift,expected_search_id=first['search_id'],**PATHS)
        for change in (dict(offset='01'),dict(offset='10'),dict(limit=129),dict(scan_limit=4097),dict(choices=[dict(slot=True,items=[{}])]),
                       dict(choices=[dict(slot=0,items=[{},{}])]),dict(choices=[dict(slot=0,items=[dict(id=True)])])):
            with self.subTest(change=change),self.assertRaises(ValueError):
                gear.generate_gear_builds(**(request()|change),**PATHS)
        for value in ({'id':0},{'enchant':1},{'id':1,'extra':1},{'id':1,'enchant':True}):
            with self.assertRaises(ValueError): gear.item_spec(value)
        with patch.object(gear,'RULES_SHA256','0'*64),self.assertRaises(ValueError): gear.generate_gear_builds(**request(),**PATHS)
        with self.assertRaises(ValueError): gear.dispatch('generate_gear_builds',request()|dict(probe_path='PRIVATE'),CATALOGUE,PROBE)

    def test_actual_cli_and_stdio_mcp(self):
        expected = gear.generate_gear_builds(**request(),**PATHS)
        args = ['--catalogue',str(CATALOGUE),'--probe',str(PROBE)]
        with tempfile.TemporaryDirectory(prefix='gear-build-') as folder:
            path = Path(folder)/'input.json'; path.write_text(json.dumps(request()))
            cli = [sys.executable,'-B','-m','tools.gear_builds','generate',*args,'--input',str(path)]
            run = subprocess.run(cli,cwd=ROOT,capture_output=True,text=True,check=True,timeout=15)
            self.assertEqual(json.loads(run.stdout),expected)
            path.write_text('{"context":"PRIVATE","context":{}}')
            bad = subprocess.run(cli,cwd=ROOT,capture_output=True,text=True,timeout=15)
            self.assertNotEqual(bad.returncode,0); self.assertNotIn('PRIVATE',bad.stderr)
            path.write_bytes(b' '*(512*1024+1))
            self.assertNotEqual(subprocess.run(cli,cwd=ROOT,capture_output=True,timeout=15).returncode,0)
        async def caller():
            params = StdioServerParameters(command=sys.executable,args=['-B','-m','tools.gear_builds','serve',*args],
                                           env={**os.environ,'PYTHONPATH':str(ROOT)},cwd=str(ROOT))
            async with stdio_client(params) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize(); tools = await session.list_tools()
                    self.assertEqual({t.name for t in tools.tools},{'validate_gear_build','generate_gear_builds'})
                    reply = await session.call_tool('generate_gear_builds',request())
                    self.assertFalse(reply.isError); self.assertEqual(json.loads(reply.content[0].text),expected)
                    bad = await session.call_tool('generate_gear_builds',request()|dict(catalogue_path='PRIVATE'))
                    self.assertTrue(bad.isError); self.assertNotIn('PRIVATE',bad.content[0].text)
        asyncio.run(caller())

    @unittest.skipUnless(os.environ.get('AZEROTH_EQUIPMENT_PROBE'),'Explicit native equipment probe needed')
    def test_real_native_modifier_conversion_and_failure(self):
        for slot,item in ((6,dict(id=720,enchant=15)),(14,dict(id=866,randomSuffix=12718))):
            result = gear.validate_gear_build(context(),equipment(**{str(slot):item}),inventory(item),**PATHS)
            self.assertTrue(result['valid'],result); self.assertFalse(result['native_legality_verified'])
        bad = dict(id=754,enchant=15)
        result = gear.validate_gear_build(context(),equipment(**{'14':bad}),inventory(bad),**PATHS)
        self.assertEqual(result['status'],'unresolved'); self.assertFalse(result['valid'])
        real_run = subprocess.run
        with tempfile.TemporaryDirectory(prefix='gear-probe-tamper-') as folder:
            copied = Path(folder)/'equipment-probe.exe'; shutil.copyfile(PROBE,copied)
            def tamper_after_launch(*args,**kwargs):
                result = real_run(*args,**kwargs)
                with copied.open('ab') as stream:
                    stream.write(b'changed after process start')
                return result
            item = dict(id=720,enchant=15)
            with patch.object(gear.subprocess,'run',side_effect=tamper_after_launch):
                result = gear.validate_gear_build(context(),equipment(**{'6':item}),inventory(item),catalogue_path=CATALOGUE,probe_path=copied)
            self.assertEqual(result['status'],'unresolved'); self.assertEqual(result['unresolved'],[dict(code='probe_untrusted')])


if __name__=='__main__':
    unittest.main()
