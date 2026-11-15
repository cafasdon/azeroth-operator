"""Talent constraints, generated references and actual offline CLI/MCP callers."""
import asyncio
from copy import deepcopy
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools import talent_builds as talents, joint_build_optimizer as joint
from tests.benchmark_joint_build_optimizer import fixture

ROOT = Path(__file__).resolve().parents[1]


def request():
    player = json.loads((ROOT/'tests/fixtures/forever-hunter-bm.json').read_bytes())['player']
    return dict(class_name='hunter',level=60,base_talents=player['talentsString'],
                choices=[dict(definition_id=134422,ranks=[4,5]),dict(definition_id=134438,ranks=[3,4])])


class TalentBuildTests(unittest.TestCase):
    def test_joint_rank_domains_retain_only_two_valid_combinations(self):
        value = request(); original = deepcopy(value)
        result = talents.generate_talent_builds(**value)
        # Independent Cartesian reconstruction: preserve all other ranks.
        expected = []
        for first,second in itertools.product([4,5],[3,4]):
            string = str(first)+str(second)+value['base_talents'][2:]
            if (first,second) in ((4,4),(5,3)):
                expected.append(string)
        self.assertEqual([r['talents_string'] for r in result['accepted']],expected)
        self.assertEqual(result['examined_count'],4)
        self.assertEqual(result['rejected_count'],2)
        self.assertEqual(result['rejection_reasons'],dict(row_gate=1,point_budget=1))
        self.assertTrue(result['declared_matrix_complete'])
        self.assertEqual(value,original)
        for row in result['accepted']:
            entry = result['references'][row['reference_sha256']]
            self.assertEqual(entry,dict(category='talents',value=row['talents_string']))
            self.assertEqual(joint.planner.digest(entry),row['reference_sha256'])
        self.assertFalse(result['native_legality_verified'])
        self.assertFalse(result['full_talent_space_covered'])

    def test_windows_resume_drift_and_no_valid_result_axis(self):
        full = talents.generate_talent_builds(**request())
        first = talents.generate_talent_builds(**request(),limit=1)
        self.assertEqual(first['next_offset'],'2')
        second = talents.generate_talent_builds(**request(),offset='2',expected_search_id=first['search_id'])
        self.assertEqual(first['accepted']+second['accepted'],full['accepted'])
        self.assertFalse(second['declared_matrix_complete'])
        empty = talents.generate_talent_builds(**request(),scan_limit=1)
        self.assertIsNone(empty['axis']); self.assertEqual(empty['next_offset'],'1')
        terminal = talents.generate_talent_builds(**request(),offset='4')
        self.assertEqual(terminal['examined_count'],0); self.assertIsNone(terminal['next_offset'])
        drift = request(); drift['choices'][0]['ranks'].reverse()
        with self.assertRaises(ValueError): talents.generate_talent_builds(**drift,expected_search_id=full['search_id'])

    def test_reference_rules_all_classes_rank_budget_and_prerequisite(self):
        for name in talents.CLASSES:
            for level in (1,9,10,60):
                self.assertTrue(talents.validate_talent_build(name,level,'--')['valid'])
        for filename in ('forever-hunter-bm.json','forever-hunter-mm.json'):
            string = json.loads((ROOT/'tests/fixtures'/filename).read_bytes())['player']['talentsString']
            self.assertTrue(talents.validate_talent_build('hunter',60,string)['valid'])
            self.assertFalse(talents.validate_talent_build('hunter',59,string)['valid'])
        value = request()['base_talents']
        result = talents.validate_talent_build('hunter',60,'9'+value[1:])
        self.assertFalse(result['valid']); self.assertIn('rank_limit',[v['code'] for v in result['violations']])
        trees = talents.rules()['classes']['hunter']
        for tree_index,tree in enumerate(trees):
            for index,node in enumerate(tree):
                if node['prerequisite'] is not None:
                    points = [['0']*len(t) for t in trees]; points[tree_index][index]='1'
                    report = talents.validate_talent_build('hunter',60,'-'.join(''.join(t) for t in points))
                    self.assertIn('prerequisite',[v['code'] for v in report['violations']])

    def test_strict_arguments_and_pinned_artifact(self):
        for change in (dict(level=True),dict(level=61),dict(class_name='unknown'),dict(base_talents='x'),
                       dict(offset='01'),dict(offset='5'),dict(limit=0),dict(scan_limit=4097),
                       dict(choices=[dict(definition_id=True,ranks=[0])]),
                       dict(choices=[dict(definition_id=134422,ranks=[True])]),
                       dict(choices=[dict(definition_id=134422,ranks=[6])]),
                       dict(choices=[dict(definition_id=134422,ranks=[1,1])])):
            with self.subTest(change=change),self.assertRaises((ValueError,TypeError)):
                talents.generate_talent_builds(**(request()|change))
        with self.assertRaises(ValueError): talents.dispatch('generate_talent_builds',request()|dict(path='PRIVATE'))
        with patch.object(talents,'RULES_SHA256','0'*64),self.assertRaises(ValueError):
            talents.generate_talent_builds(**request())

    def test_actual_cli_and_stdio_mcp(self):
        with tempfile.TemporaryDirectory(prefix='talent-build-') as folder:
            path = Path(folder)/'input.json'; path.write_text(json.dumps(request()))
            run = subprocess.run([sys.executable,'-B','-m','tools.talent_builds','generate','--input',str(path)],
                                 cwd=ROOT,capture_output=True,text=True,check=True,timeout=15)
            expected = talents.generate_talent_builds(**request())
            self.assertEqual(json.loads(run.stdout),expected)
            path.write_text('{"class_name":"PRIVATE","class_name":"hunter"}')
            bad = subprocess.run([sys.executable,'-B','-m','tools.talent_builds','generate','--input',str(path)],
                                 cwd=ROOT,capture_output=True,text=True,timeout=15)
            self.assertNotEqual(bad.returncode,0); self.assertNotIn('PRIVATE',bad.stderr)
        async def caller():
            params = StdioServerParameters(command=sys.executable,args=['-B','-m','tools.talent_builds','serve'],
                                          env={**os.environ,'PYTHONPATH':str(ROOT)},cwd=str(ROOT))
            async with stdio_client(params) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    listing = await session.list_tools()
                    self.assertEqual({t.name for t in listing.tools},{'validate_talent_build','generate_talent_builds'})
                    reply = await session.call_tool('generate_talent_builds',request())
                    self.assertFalse(reply.isError); self.assertEqual(json.loads(reply.content[0].text),expected)
                    bad = await session.call_tool('generate_talent_builds',request()|dict(path='PRIVATE'))
                    self.assertTrue(bad.isError); self.assertNotIn('PRIVATE',bad.content[0].text)
        asyncio.run(caller())

    @unittest.skipUnless(os.environ.get('AZEROTH_JOINT_ENGINE'),'Explicit local native simulator needed')
    def test_generated_talents_feed_actual_native_joint_evaluator(self):
        generated = talents.generate_talent_builds(**request())
        manifest = fixture(20261115,2)
        manifest.update(axes=[generated['axis']],references=generated['references'])
        manifest['case']['constraints_sha256']=joint.planner.digest(dict(talent_search=generated['search_id']))
        result = joint.evaluate_joint_build_case(manifest,Path(os.environ['AZEROTH_JOINT_ENGINE']))
        self.assertEqual(result['evaluated_coordinates'],2)
        self.assertTrue(all(r['mean_dps']>0 for r in result['results']))
        self.assertEqual({r['selection']['talents'] for r in result['results']},set(generated['references']))
        self.assertFalse(result['global_optimum_verified'])


if __name__=='__main__':
    unittest.main()
