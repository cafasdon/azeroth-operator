"""Collect and resume explicit offline plans without executing stored instructions."""
import asyncio
from contextlib import closing
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from test_merge_gear_windows import report
from tools import collect_forever_gear_windows as controller
from tools.journal_forever_gear_windows import append_report, journal_status, journal_window
from tools.merge_forever_gear_windows import merge_reports


class CollectGearWindowTests(unittest.TestCase):
    def test_one_real_process_matches_standalone_windows_and_closes_on_failure(self):
        import mcp.client.stdio as sdk
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file():self.skipTest('Requires pinned local catalogue; no download')
        binary=(root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder:
            base=Path(folder);value=fixture()
            value['client'].update(family='forever',build='70862',interface=16001)
            value['sections']['bags']={'0':{str(i):{'item_id':item,'count':1,'link':f'item:{item}:0:0:0:0:0:0'}
                for i,item in enumerate([4549,4550],1)}}
            key=SnapshotStore(base).ingest(value);template,_,_=inputs()
            template_path=base/'template.json';template_path.write_text(json.dumps(template))
            originals={path:path.read_bytes() for path in [template_path,base/(key+'.json')]}
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            argv=['--mcp',str(binary),'--snapshot-id',key,'--slot','finger1','--template',str(template_path),
                '--journal',str(base/'plans.sqlite'),'--collapse-identical-items','--retain-equipment-specs',
                '--window-size','1','--max-windows','8']
            processes=[];create=sdk._create_platform_compatible_process
            async def tracked(*a,**kw):
                process=await create(*a,**kw);processes.append(process);return process
            async def run():return await controller.collect_windows(controller.parser().parse_args(argv))
            with patch.dict(os.environ,env),patch.object(sdk,'_create_platform_compatible_process',tracked):
                result=asyncio.run(run())
                self.assertEqual(result['windows_inserted'],3)
                self.assertTrue(result['progress']['reported_coordinate_coverage_complete'])
                self.assertEqual(len(processes),1)
                self.assertIsNotNone(processes[0].returncode)
                repeated=asyncio.run(run())
                self.assertEqual(repeated['windows_attempted'],0);self.assertEqual(len(processes),1)
                with closing(sqlite3.connect(base/'plans.sqlite')) as db:
                    windows=[json.loads(row[0]) for row in db.execute('SELECT report FROM windows ORDER BY start_key')]
                with tempfile.TemporaryFile(mode='w+') as errlog:
                    for offset,saved in enumerate(windows):
                        async def standalone():
                            return await controller.ranker.collect(binary,key,['finger1'],errlog,
                                collapse_identical=True,can_dual_wield=True,combination_offset=offset,combination_limit=1)
                        drafts,skipped,coverage,_,_,_,search=asyncio.run(standalone())
                        expected=controller.ranker.plan_report(drafts,True,True)
                        self.assertEqual(saved['resolved_drafts'],expected['resolved_drafts'])
                        self.assertEqual(saved['gear_search'],search)
                        self.assertEqual(saved['skipped'],skipped);self.assertEqual(saved['storage_coverage'],coverage)
                self.assertEqual(len(processes),4)
                self.assertTrue(all(process.returncode is not None for process in processes))
                argv[argv.index('--journal')+1]=str(base/'interrupted.sqlite')
                collect=controller.ranker.collect;calls=[0]
                async def fail_second(*a,**kw):
                    calls[0]+=1
                    if calls[0]==2:raise ValueError('injected window failure')
                    return await collect(*a,**kw)
                with patch.object(controller.ranker,'collect',fail_second):
                    with self.assertRaisesRegex(ValueError,'injected window failure'):asyncio.run(run())
                self.assertEqual(journal_status(base/'interrupted.sqlite')['window_count'],1)
                self.assertEqual(len(processes),5)
                self.assertTrue(all(process.returncode is not None for process in processes))
                resumed=asyncio.run(run())
                self.assertEqual(resumed['windows_inserted'],2)
                self.assertTrue(resumed['progress']['reported_coordinate_coverage_complete'])
                self.assertEqual(len(processes),6)
                self.assertTrue(all(process.returncode is not None for process in processes))
                argv[argv.index('--journal')+1]=str(base/'startup.sqlite')
                argv.extend(['--time-budget-seconds','1'])
                handshake={'started':False,'cancelled':False}
                async def stalled_initialize(*a,**kw):
                    handshake['started']=True
                    try:await asyncio.sleep(30)
                    finally:handshake['cancelled']=True
                with patch.object(controller.ranker.ClientSession,'initialize',stalled_initialize):
                    with self.assertRaises(Exception):asyncio.run(run())
                self.assertTrue(handshake['started']);self.assertTrue(handshake['cancelled'])
                self.assertEqual(len(processes),7)
                self.assertTrue(all(process.returncode is not None for process in processes))
                self.assertEqual((base/'startup.sqlite').stat().st_size,0)
                # Thirty seconds of handshake time leaves fifteen of the existing 45-second window cap.
                argv[argv.index('--journal')+1]=str(base/'window-cap.sqlite')
                argv.extend(['--time-budget-seconds','180','--max-windows','1'])
                from contextlib import asynccontextmanager
                clock=controller.monotonic;skew=[0.0];caps=[]
                connection=controller.ranker.collection_connection;wait_for=asyncio.wait_for
                @asynccontextmanager
                async def delayed_connection(*a,**kw):
                    async with connection(*a,**kw) as session:
                        if len(a)<3 and kw.get('existing') is None:skew[0]+=30
                        yield session
                async def measured_wait(coroutine,timeout):
                    if getattr(coroutine,'cr_code',None) is collect.__code__:caps.append(timeout)
                    return await wait_for(coroutine,timeout=timeout)
                with patch.object(controller,'monotonic',lambda:clock()+skew[0]), \
                        patch.object(controller.ranker,'collection_connection',delayed_connection), \
                        patch.object(asyncio,'wait_for',measured_wait):
                    capped=asyncio.run(run())
                self.assertEqual(capped['windows_inserted'],1)
                self.assertEqual(len(caps),1);self.assertGreater(caps[0],14);self.assertLess(caps[0],16)
                self.assertEqual(len(processes),8)
                self.assertTrue(all(process.returncode is not None for process in processes))
            self.assertEqual(originals,{path:path.read_bytes() for path in originals})

    def test_real_module_covers_product_above_old_request_limit(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        db = json.loads(catalogue.read_text())
        ids = [row['id'] for row in db['items'] if row.get('type') == 11
               and not row.get('randomSuffixOptions') and not row.get('classAllowlist')
               and not row.get('requiredProfession')][:32]
        self.assertEqual(len(set(ids)), 32)
        with tempfile.TemporaryDirectory() as folder:
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['bags'] = {'0': {str(i): {'item_id': item, 'count': 1,
                'link': f'item:{item}:0:0:0:0:0:0'} for i, item in enumerate(ids, 1)}}
            key = SnapshotStore(Path(folder)).ingest(value)
            template, _, _ = inputs()
            template_path = Path(folder) / 'PRIVATE_template.json'
            template_path.write_text(json.dumps(template))
            journal = Path(folder) / 'PRIVATE_large.sqlite'
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder,
                   'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            argv = [sys.executable, '-m', 'tools.collect_forever_gear_windows',
                '--mcp', str(binary), '--snapshot-id', key, '--slot', 'finger1', '--slot', 'finger2',
                '--template', str(template_path), '--journal', str(journal), '--collapse-identical-items',
                '--window-size', '128', '--max-drafts', '128', '--time-budget-seconds', '300']
            def launch(windows):
                process = subprocess.run([*argv, '--max-windows', str(windows)], cwd=root, env=env,
                    capture_output=True, text=True, timeout=330)
                self.assertEqual(process.returncode, 0, process.stderr)
                self.assertNotIn('PRIVATE', process.stdout + process.stderr)
                return json.loads(process.stdout)
            first = launch(1)
            self.assertEqual(first['progress']['total_combinations_decimal'], str(33**2))
            self.assertEqual(first['progress']['covered_coordinate_count'], 128)
            self.assertFalse(first['progress']['reported_coordinate_coverage_complete'])
            resumed = launch(8)
            progress = resumed['progress']
            self.assertEqual(resumed['windows_inserted'], 8)
            self.assertEqual(progress['window_count'], 9)
            self.assertEqual(progress['covered_coordinate_count'], 1089)
            self.assertTrue(progress['reported_coordinate_coverage_complete'])
            self.assertEqual(progress['uncovered_ranges'], [])
            self.assertIsNone(progress['next_uncovered_offset_decimal'])
            self.assertEqual(first['collection_binding_sha256'], resumed['collection_binding_sha256'])
            with closing(sqlite3.connect(journal)) as connection:
                rows = connection.execute('SELECT start_key,end_key,report FROM windows ORDER BY start_key').fetchall()
            # Independent arithmetic oracle for the full 33 x 33 coordinate product.
            intervals = [(int(start), int(end)) for start, end, _ in rows]
            self.assertEqual(intervals, [(start, min(start + 128, 1089)) for start in range(0, 1089, 128)])
            coordinates = [coordinate for start, end in intervals for coordinate in range(start, end)]
            self.assertEqual(coordinates, list(range(33**2)))
            for _, _, payload in rows:
                report_value = json.loads(payload)
                self.assertFalse(report_value['best_in_slot_verified'])
                self.assertLessEqual(len(report_value['resolved_drafts']), 129)
            for start, end in intervals:
                inspection = journal_window(journal, start)
                self.assertTrue(inspection['coordinate_outcomes_available'])
                self.assertEqual([row['coordinate_decimal'] for row in inspection['coordinate_outcomes']],
                                 [str(i) for i in range(start, end)])
                for row in inspection['coordinate_outcomes']:
                    first_choice, second_choice = divmod(int(row['coordinate_decimal']), 33)
                    if first_choice == second_choice and first_choice > 0:
                        self.assertEqual(row['status'], 'excluded')
                        self.assertEqual(row['reason'], 'reused_source_location')
                    else:
                        self.assertNotEqual(row.get('reason'), 'reused_source_location')
                self.assertNotIn('selections', json.dumps(inspection))
            before = journal.read_bytes()
            complete = launch(1)
            self.assertEqual(complete['windows_attempted'], 0)
            self.assertEqual(journal.read_bytes(), before)
            for flag in ['best_in_slot_verified', 'feasible_set_coverage_verified',
                         'simulation_performed', 'game_execution']:
                self.assertFalse(complete[flag])

    def test_binding_guards_across_merger_and_journal(self):
        first, second = report(0, ['b' * 64]), report(4, ['b' * 64])
        first['collection_binding_sha256'] = 'a' * 64
        second['collection_binding_sha256'] = 'a' * 64
        self.assertEqual(merge_reports([first, second])['collection_binding_sha256'], 'a' * 64)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'bound.sqlite'
            append_report(path, first)
            before = path.read_bytes()
            for binding in [None, 'c' * 64, True, 'PRIVATE']:
                bad = deepcopy(second)
                bad['collection_binding_sha256'] = binding
                with self.assertRaises(ValueError):
                    merge_reports([first, bad])
                with self.assertRaises(ValueError):
                    append_report(path, bad)
                self.assertEqual(path.read_bytes(), before)
            append_report(path, second)
            self.assertEqual(journal_status(path)['collection_binding_sha256'], 'a' * 64)

    def test_real_module_resume_identity_and_no_recollection(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            base['sections']['bags'] = {'0': {str(i): {'item_id': item, 'count': 1,
                'link': f'item:{item}:0:0:0:0:0:0'} for i, item in enumerate([4549, 4550], 1)}}
            key = SnapshotStore(Path(folder)).ingest(base)
            template, _, _ = inputs()
            template_path = Path(folder) / 'PRIVATE_template.json'
            template_path.write_text(json.dumps(template))
            original_template = template_path.read_bytes()
            journal = Path(folder) / 'PRIVATE_progress.sqlite'
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            argv = ['--mcp', str(binary), '--snapshot-id', key, '--slot', 'finger1', '--slot', 'finger2',
                '--template', str(template_path), '--journal', str(journal), '--collapse-identical-items',
                '--window-size', '4', '--max-windows', '1']
            def launch(extra=(), success=True):
                process = subprocess.run([sys.executable, '-m', 'tools.collect_forever_gear_windows', *argv, *extra],
                    cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(process.returncode, 0 if success else 1, process.stderr)
                self.assertNotIn('PRIVATE', process.stdout + process.stderr)
                return json.loads(process.stdout) if success else None
            first = launch()
            self.assertEqual(first['windows_inserted'], 1)
            self.assertEqual(first['progress']['next_uncovered_offset_decimal'], '4')
            before = journal.read_bytes()
            with patch.dict(os.environ, env):
                async def stalled(*args, **kwargs):
                    await asyncio.sleep(30)
                budget_args = controller.parser().parse_args([*argv, '--time-budget-seconds', '1'])
                with patch('tools.rank_forever_slot.collect', stalled):
                    with self.assertRaises(asyncio.TimeoutError):
                        asyncio.run(controller.collect_windows(budget_args))
            self.assertEqual(journal.read_bytes(), before)
            template_path.write_text(json.dumps({**template, 'PRIVATE_EXTRA': True}))
            launch(success=False)
            self.assertEqual(journal.read_bytes(), before)
            template_path.write_bytes(original_template)
            launch(['--storage', 'bags'], success=False)
            self.assertEqual(journal.read_bytes(), before)
            resumed = launch(['--max-windows', '2', '--max-drafts', '8'])
            self.assertEqual(resumed['windows_inserted'], 2)
            self.assertTrue(resumed['progress']['reported_coordinate_coverage_complete'])
            self.assertEqual(resumed['collection_binding_sha256'], first['collection_binding_sha256'])
            self.assertFalse(resumed['best_in_slot_verified'])
            self.assertFalse(resumed['simulation_performed'])
            before = journal.read_bytes()
            complete = launch()
            self.assertEqual(complete['windows_attempted'], 0)
            self.assertEqual(journal.read_bytes(), before)
            with patch.dict(os.environ, env):
                args = controller.parser().parse_args(argv)
                with patch('tools.rank_forever_slot.collect', side_effect=AssertionError('No recollection')):
                    result = asyncio.run(controller.collect_windows(args))
                    self.assertEqual(result['windows_attempted'], 0)
                unbound = Path(folder) / 'unbound.sqlite'
                append_report(unbound, report(0, ['b' * 64]))
                bad_args = controller.parser().parse_args([*argv, '--journal', str(unbound)])
                with patch('tools.rank_forever_slot.collect', side_effect=AssertionError('No adoption')):
                    with self.assertRaises(ValueError):
                        asyncio.run(controller.collect_windows(bad_args))
                changed = Path(folder) / 'changed.sqlite'
                changed_args = controller.parser().parse_args([*argv, '--journal', str(changed)])
                original_collect = controller.ranker.collect
                async def mutate(*args, **kwargs):
                    collected = await original_collect(*args, **kwargs)
                    template_path.write_text(json.dumps({**template, 'PRIVATE_CHANGED': True}))
                    return collected
                with patch('tools.rank_forever_slot.collect', mutate):
                    with self.assertRaises(ValueError):
                        asyncio.run(controller.collect_windows(changed_args))
                self.assertEqual(changed.stat().st_size, 0)
                template_path.write_bytes(original_template)
            for extra in [['--window-size', '129'], ['--max-windows', '0'], ['--max-payload-mib', '0'],
                          ['--minimum-required-level', '5', '--maximum-required-level', '4'],
                          ['--item-id', '0']]:
                launch(extra, success=False)
