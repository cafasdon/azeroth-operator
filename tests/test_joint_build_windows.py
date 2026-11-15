"""Independent coordinate/coverage oracles and real fixed-manifest entry points."""
import asyncio
from copy import deepcopy
import inspect
import itertools
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from tests.test_joint_build_optimizer import manifest,add_axis,player,ROOT
from tools import joint_build_optimizer as joint, joint_build_windows as windows


def native_double(engine,expected_hash,requests,paired=False,tank_metrics=False,healing_metrics=False,cache=None):
    """Mirror the real adapter signature; check the real pinned request schema."""
    if expected_hash!=joint.layout.ENGINE_SHA256 or not requests:
        raise ValueError('Bad adapter call')
    log = os.environ.get('AZEROTH_WINDOW_TEST_LOG')
    if log:
        with Path(log).open('a') as stream:
            stream.write('start\n')
        time.sleep(.7)
    rows = []
    schema = joint.request_type().DESCRIPTOR
    for request in requests:
        joint.check_message(schema,request)
        p = player(request)
        score = p['equipment']['items'][0].get('id',0)+int(p.get('talentsString','0'))
        rows.append(dict(mean_dps=score,dps_standard_deviation=2,iterations=request['simOptions']['iterations'],
                         seed=request['simOptions']['randomSeed'],engine_sha256=expected_hash))
    if log:
        with Path(log).open('a') as stream:
            stream.write('end\n')
    return dict(results=rows)


def fixture_entry():
    """Test subprocess only: install the schema-checked adapter double before CLI."""
    joint.adapter.verify_engine = lambda engine,expected_hash: None
    joint.adapter.compare = native_double
    windows.main()


def large_manifest(size=5):
    value = manifest(); value['axes']=[]; value['references']={}
    add_axis(value,'gear',[dict(items=[dict(id=100+i)]+[{} for _ in range(16)],preserveSlots=True) for i in range(size)])
    add_axis(value,'talents',[str(i) for i in range(size)])
    add_axis(value,'timing',[dict(hpPercentForDefensives=i/size) for i in range(size)])
    return value


class JointWindowTests(unittest.TestCase):
    def page(self,value,offset='0',limit=64):
        with patch.object(joint.adapter,'verify_engine'),patch.object(joint.adapter,'compare',side_effect=native_double) as run:
            result = windows.evaluate_joint_build_window(value,ROOT/'unused-engine.exe',offset,limit)
        return result,run

    def test_adapter_double_real_signature_and_original_full_cap(self):
        self.assertEqual(inspect.signature(native_double),inspect.signature(joint.adapter.compare))
        with patch.object(joint.adapter,'verify_engine'),patch.object(joint.adapter,'compare',side_effect=native_double):
            result = joint.evaluate_joint_build_case(manifest(),ROOT/'unused-engine.exe')
        self.assertNotIn('kind',result); self.assertTrue(result['declared_matrix_complete'])
        with self.assertRaises(ValueError): joint.preflight(large_manifest())
        for limit in (0,65,True):
            with self.assertRaises(ValueError): joint.preflight(large_manifest(),window_mode=True,limit=limit)
        with self.assertRaises(ValueError): joint.preflight(manifest(),offset='1')

    def test_independent_cartesian_pages_identity_and_terminal(self):
        value = large_manifest(); original = deepcopy(value)
        first,_ = self.page(value,limit=64); second,_ = self.page(value,offset='64',limit=64)
        expected = list(itertools.product(*(a['candidates'] for a in value['axes'])))
        rows = sorted(first['results']+second['results'],key=lambda r:int(r['ordinal_decimal']))
        self.assertEqual([tuple(r['selection'][a['name']] for a in value['axes']) for r in rows],expected)
        self.assertEqual(first['next_offset'],'64'); self.assertIsNone(second['next_offset'])
        self.assertEqual(first['search_id'],second['search_id']); self.assertEqual(first['total_decimal'],'125')
        self.assertFalse(first['declared_matrix_complete']); self.assertFalse(second['declared_matrix_complete'])
        for row in rows:
            binding = dict(search_id=first['search_id'],ordinal_decimal=row['ordinal_decimal'],selection=row['selection'])
            self.assertEqual(row['candidate_sha256'],joint.planner.digest(binding))
        tail,run = self.page(value,offset='125')
        run.assert_not_called(); self.assertEqual(tail['results'],[]); self.assertFalse(tail['simulation_performed'])
        self.assertFalse(tail['declared_matrix_complete']); self.assertIsNone(tail['next_offset'])
        self.assertEqual(value,original)

    def test_incremental_top_k_gaps_ties_and_count_every_coordinate(self):
        value = large_manifest(); a,_ = self.page(value,offset='0',limit=64); b,_ = self.page(value,offset='64',limit=61)
        original = deepcopy((value,a,b))
        one = windows.merge_joint_build_windows(value,[b],top_k=7)
        self.assertEqual(one['next_offset'],'0'); self.assertEqual(one['reported_evaluated_decimal'],'61')
        final = windows.merge_joint_build_windows(value,[a],one,top_k=7)
        expected = sorted(a['results']+b['results'],key=lambda r:(-r['mean_dps'],int(r['ordinal_decimal'])))[:7]
        self.assertEqual([r['ordinal_decimal'] for r in final['top_results']],[r['ordinal_decimal'] for r in expected])
        self.assertEqual(final['coverage_ranges'],[dict(start_decimal='0',end_decimal='125')])
        self.assertEqual(final['reported_evaluated_decimal'],'125'); self.assertTrue(final['reported_matrix_complete'])
        self.assertTrue(final['top_k_truncated']); self.assertIsNone(final['next_offset'])
        self.assertFalse(final['simulation_performed']); self.assertFalse(final['global_optimum_verified'])
        self.assertIn('unauthenticated',final['receipt_trust']); self.assertEqual((value,a,b),original)
        gapped,_ = self.page(value,offset='70',limit=5)
        summary = windows.merge_joint_build_windows(value,[gapped,a],top_k=7)
        self.assertEqual(summary['next_offset'],'64'); self.assertEqual(len(summary['coverage_ranges']),2)
        # Distinct schema spellings produce separate coordinates but one request.
        equivalent = manifest(); add_axis(equivalent,'timing',[{},dict(hpPercentForDefensives=0)])
        page,_ = self.page(equivalent)
        self.assertEqual(page['unique_requests'],4); self.assertEqual(page['evaluated_coordinates'],8)
        self.assertEqual(windows.merge_joint_build_windows(equivalent,[page])['reported_evaluated_decimal'],'8')
        bad = deepcopy(page)
        repeated = {}
        for row in bad['results']:
            if row['request_sha256'] in repeated:
                row['dps_standard_deviation']+=1; break
            repeated[row['request_sha256']]=row
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(equivalent,[bad])
        first,_ = self.page(equivalent,limit=1); repeat,_ = self.page(equivalent,offset='1',limit=1)
        repeat['results'][0]['dps_standard_deviation']+=1
        prior = windows.merge_joint_build_windows(equivalent,[first],top_k=1)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(equivalent,[repeat],prior,top_k=1)

    def test_bad_receipts_and_previous_summary_never_pass(self):
        value = manifest(); page,_ = self.page(value,limit=2); rest,_ = self.page(value,offset='2',limit=2)
        summary = windows.merge_joint_build_windows(value,[page],top_k=2)
        for receipt in ([],[page,page],[page,rest,page]):
            with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,receipt)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[page],summary,top_k=2)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[rest],summary,top_k=1)
        changes = [lambda p:p.update(search_id='0'*64),lambda p:p.update(evaluated_coordinates=True),lambda p:p.update(window_complete=False),
                   lambda p:p.update(global_optimum_verified=True),lambda p:p['results'].pop(),lambda p:p.update(total_decimal='5'),
                   lambda p:p['results'][0].update(ordinal_decimal='3'),lambda p:p['results'][0].update(request_sha256='0'*64),
                   lambda p:p['results'][0].update(seed='2'),lambda p:p['results'][0].update(iterations=True),
                   lambda p:p['results'][0].update(mean_dps=True),lambda p:p['results'][0].update(dps_standard_deviation=-1),
                   lambda p:p['results'][0].update(rank=99),lambda p:p['results'][0].update(extra='PRIVATE')]
        for mutate in changes:
            bad = deepcopy(page); mutate(bad)
            with self.assertRaises((ValueError,KeyError)): windows.merge_joint_build_windows(value,[bad])
        for score in (float('nan'),float('inf'),-1,'1'):
            bad = deepcopy(page); bad['results'][0]['mean_dps']=score
            with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[bad])
        edits = [lambda s:s.update(reported_evaluated_decimal='3'),lambda s:s.update(reported_matrix_complete=True),
                 lambda s:s.update(next_offset=None),lambda s:s.update(top_k_truncated=True),lambda s:s.update(simulation_performed=True),
                 lambda s:s.update(receipt_trust='Trusted'),lambda s:s['top_results'][0].update(candidate_sha256='0'*64),
                 lambda s:s['top_results'][0].update(ordinal_decimal='3'),lambda s:s['coverage_ranges'].append(dict(start_decimal='1',end_decimal='3')),
                 lambda s:s['top_results'].pop(),lambda s:s.update(extra='PRIVATE')]
        for mutate in edits:
            bad = deepcopy(summary); mutate(bad)
            with self.assertRaises((ValueError,KeyError)): windows.merge_joint_build_windows(value,[rest],bad,top_k=2)
        drift = deepcopy(value); drift['base_request']['simOptions']['randomSeed']='2'
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(drift,[page])

    def test_interval_input_and_native_iteration_budgets(self):
        value = large_manifest(12); first,_ = self.page(value,limit=1)
        prior = windows.merge_joint_build_windows(value,[first],top_k=1)
        # A previous receipt can report coverage; it cannot authenticate discarded rows.
        prior['coverage_ranges']=[dict(start_decimal=str(i*2),end_decimal=str(i*2+1)) for i in range(512)]
        prior.update(reported_evaluated_decimal='512',next_offset='1',top_k_truncated=True)
        extra,_ = self.page(value,offset='1024',limit=1)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[extra],prior,top_k=1)
        adjacent,_ = self.page(value,offset='1023',limit=1)
        result = windows.merge_joint_build_windows(value,[adjacent],prior,top_k=1)
        self.assertEqual(len(result['coverage_ranges']),512)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[first]*33)
        oversized = deepcopy(first); oversized['coverage']='x'*(256*1024)
        with self.assertRaises(ValueError): windows.merge_joint_build_windows(value,[oversized])
        costly = large_manifest(); costly['base_request']['simOptions']['iterations']=5000
        with patch.object(joint.adapter,'verify_engine') as verify,patch.object(joint.adapter,'compare') as run:
            with self.assertRaises(ValueError): windows.evaluate_joint_build_window(costly,ROOT/'unused.exe')
        verify.assert_not_called(); run.assert_not_called()

    def test_bounded_huge_tail_and_local_validation_only(self):
        value = large_manifest(128)
        add_axis(value,'rotation',[dict(type='TypeAPL',priorityList=[dict(action=dict(startAttack={}))],prepullActions=[dict(doAtValue=dict(const=dict(val=str(-i-1))),action=dict(startAttack={}))]) for i in range(128)])
        add_axis(value,'consumables',[dict(foodId=i) for i in range(128)])
        total = 128**5
        tail,run = self.page(value,offset=str(total-3),limit=64)
        self.assertEqual(tail['evaluated_coordinates'],3); self.assertEqual(len(run.call_args.args[2]),3)
        self.assertEqual(tail['total_decimal'],str(total)); self.assertFalse(tail['declared_matrix_complete'])
        summary = windows.merge_joint_build_windows(value,[tail],top_k=2)
        self.assertEqual(summary['reported_evaluated_decimal'],'3'); self.assertEqual(summary['next_offset'],'0')
        self.assertLess(len(joint.planner.encoded(summary)),256*1024)
        # An invalid reference beyond this page fails only when its coordinate is selected.
        invalid = manifest(); axis = invalid['axes'][1]; old = axis['candidates'][1]; entry = dict(category='talents',value={'bad':True})
        new = joint.planner.digest(entry); del invalid['references'][old]; invalid['references'][new]=entry; axis['candidates'][1]=new
        self.page(invalid,limit=1)
        with self.assertRaises((ValueError,TypeError)): self.page(invalid,offset='1',limit=1)

    def test_actual_cli_prepare_page_merge_and_input_guards(self):
        with tempfile.TemporaryDirectory(prefix='joint-window-cli-') as folder:
            folder = Path(folder); path = folder/'manifest.json'; path.write_text(joint.planner.encoded(manifest()))
            def cli(*args,stub=False):
                start = [sys.executable,'-B','-c','from tests.test_joint_build_windows import fixture_entry;fixture_entry()'] if stub else [sys.executable,'-B','-m','tools.joint_build_windows']
                return subprocess.run([*start,*args,'--input',str(path)],cwd=ROOT,capture_output=True,text=True,timeout=30)
            prepare = cli('prepare'); self.assertEqual(prepare.returncode,0,prepare.stderr)
            identity = json.loads(prepare.stdout)['search_id']; self.assertEqual(json.loads(prepare.stdout)['total_decimal'],'4')
            page = cli('page','--engine',str(ROOT/'unused-engine.exe'),'--limit','2','--expected-search-id',identity,stub=True)
            self.assertEqual(page.returncode,0,page.stderr); receipt = folder/'window.json'; receipt.write_text(page.stdout)
            merge = cli('merge','--windows',str(receipt),'--top-k','2')
            self.assertEqual(merge.returncode,0,merge.stderr); self.assertEqual(json.loads(merge.stdout)['next_offset'],'2')
            receipt.write_text('{"PRIVATE":1,"PRIVATE":2}')
            bad = cli('merge','--windows',str(receipt)); self.assertNotEqual(bad.returncode,0); self.assertNotIn('PRIVATE',bad.stderr)
            receipt.write_bytes(b' '*(256*1024+1)); self.assertNotEqual(cli('merge','--windows',str(receipt)).returncode,0)
            path.write_text('{"PRIVATE":1,"PRIVATE":2}'); self.assertNotEqual(cli('prepare').returncode,0)

    def test_actual_mcp_fixed_paths_busy_lock_and_cancellation(self):
        with tempfile.TemporaryDirectory(prefix='joint-window-mcp-') as folder:
            folder = Path(folder); path = folder/'manifest.json'; path.write_text(joint.planner.encoded(manifest())); log = folder/'native.log'
            async def caller():
                params = StdioServerParameters(command=sys.executable,cwd=str(ROOT),
                    args=['-B','-c','from tests.test_joint_build_windows import fixture_entry;fixture_entry()','serve','--input',str(path),'--engine',str(ROOT/'unused-engine.exe')],
                    env={**os.environ,'AZEROTH_WINDOW_TEST_LOG':str(log)})
                async with stdio_client(params) as (read,write),ClientSession(read,write) as session:
                    await session.initialize(); listing = await session.list_tools()
                    self.assertEqual({t.name for t in listing.tools},{'evaluate_joint_build_window','merge_joint_build_windows'})
                    identity = joint.preflight(manifest())[0]['search_id']; args = dict(expected_search_id=identity,limit=2)
                    first = asyncio.create_task(session.call_tool('evaluate_joint_build_window',args))
                    for _ in range(100):
                        if log.exists() and 'start' in log.read_text(): break
                        await asyncio.sleep(.02)
                    self.assertTrue(log.exists())
                    busy = await session.call_tool('evaluate_joint_build_window',args); self.assertTrue(busy.isError)
                    first.cancel()
                    try: await first
                    except asyncio.CancelledError: pass
                    busy = await session.call_tool('evaluate_joint_build_window',args); self.assertTrue(busy.isError)
                    for _ in range(100):
                        if 'end' in log.read_text(): break
                        await asyncio.sleep(.02)
                    await asyncio.sleep(.05)
                    reply = await session.call_tool('evaluate_joint_build_window',args); self.assertFalse(reply.isError)
                    window = json.loads(reply.content[0].text)
                    merged = await session.call_tool('merge_joint_build_windows',dict(expected_search_id=identity,windows=[window],top_k=2))
                    self.assertFalse(merged.isError); self.assertEqual(json.loads(merged.content[0].text)['reported_evaluated_decimal'],'2')
                    for changes in ({'engine':'PRIVATE'},{'request':{}},{'expected_search_id':'0'*64}):
                        bad = await session.call_tool('evaluate_joint_build_window',args|changes)
                        self.assertTrue(bad.isError); self.assertNotIn('PRIVATE',bad.content[0].text)
            asyncio.run(asyncio.wait_for(caller(),30))
            self.assertEqual(log.read_text().splitlines(),['start','end','start','end'])


if __name__=='__main__':
    unittest.main()
