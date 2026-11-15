"""Per-comparison capability reuse keeps real engine hashes and private cache checks."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

from tools import run_forever_sim as adapter
from tools.simulation_cache import SimulationCache

ROOT = Path(__file__).resolve().parents[1]


class PositionalProbeReuseTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='positional-probe-test-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.engine = self.root / 'synthetic-engine.exe'
        self.engine.write_bytes(b'fixed synthetic engine bytes')
        self.sha = hashlib.sha256(self.engine.read_bytes()).hexdigest()
        self.cache = SimulationCache(self.root / 'cache')
        self.requests = [dict(raid=dict(parties=[dict(players=[dict(equipment=dict(
            items=[dict(id=identifier)] + [{} for _ in range(16)], preserveSlots=True))])]),
            encounter=dict(duration=30), simOptions=dict(iterations=2, randomSeed='1')) for identifier in (1, 2)]
        qualified = patch.object(adapter.layout, 'ENGINE_SHA256', self.sha)
        qualified.start()
        self.addCleanup(qualified.stop)
        self.capability = json.dumps(adapter.layout.CAPABILITY).encode()
        self.capability_calls = self.simulation_calls = 0

    def launch(self, args, *, stdin, stdout, stderr, check, timeout, cwd=None):
        """Match the two actual adapter subprocess.run calls, including keyword-only I/O."""
        self.assertEqual(args[0], str(self.engine))
        self.assertEqual(stdin, subprocess.DEVNULL)
        self.assertEqual(stderr, subprocess.DEVNULL)
        self.assertIs(check, True)
        self.assertGreater(timeout, 0)
        if args[1:] == ['azeroth-equipment-contract']:
            self.assertIsNone(cwd)
            self.capability_calls += 1
            stdout.write(self.capability)
        else:
            self.assertEqual(args[1:3], ['sim', '--infile'])
            self.assertEqual(stdout, subprocess.DEVNULL)
            self.assertIsNotNone(cwd)
            self.simulation_calls += 1
            request = json.loads(Path(args[3]).read_text())
            output = dict(iterationsDone=request['simOptions']['iterations'],
                          raidMetrics=dict(dps=dict(avg=100, stdev=2)))
            Path(args[5]).write_text(json.dumps(output))
        return subprocess.CompletedProcess(args, 0)

    def test_one_probe_per_batch_and_real_disk_cache_hits(self):
        with patch.object(adapter.subprocess, 'run', side_effect=self.launch):
            first = adapter.compare(self.engine, self.sha, [*self.requests, self.requests[0]], cache=self.cache)
            self.assertEqual((self.capability_calls, self.simulation_calls), (1, 2))
            self.assertEqual(first['unique_runs'], 2)
            self.assertEqual([r['cache_status'] for r in first['results']], ['miss']*3)
            second = adapter.compare(self.engine, self.sha, self.requests, cache=SimulationCache(self.cache.root))
            self.assertEqual((self.capability_calls, self.simulation_calls), (2, 2))
            self.assertEqual([r['cache_status'] for r in second['results']], ['hit', 'hit'])
            self.assertEqual([r['mean_dps'] for r in first['results'][:2]], [r['mean_dps'] for r in second['results']])
        self.assertTrue((self.cache.root/'results.sqlite3').is_file())

    def test_tamper_after_first_cache_hit_blocks_second_cache_read(self):
        with patch.object(adapter.subprocess, 'run', side_effect=self.launch):
            adapter.compare(self.engine, self.sha, self.requests, cache=self.cache)
            real_get = self.cache.get
            def tamper_after_read(key):
                result = real_get(key)
                self.assertIsNotNone(result)
                self.engine.write_bytes(b'tampered between cache hits')
                return result
            with patch.object(self.cache, 'get', side_effect=tamper_after_read) as get:
                with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                    adapter.compare(self.engine, self.sha, self.requests, cache=self.cache)
                self.assertEqual(get.call_count, 1)
            self.assertEqual((self.capability_calls, self.simulation_calls), (2, 2))

    def test_reused_probe_still_hashes_engine_and_checks_qualified_pin(self):
        verified = set()
        with patch.object(adapter.subprocess, 'run', side_effect=self.launch):
            adapter.verify_positional_engine(self.engine, self.sha, _verified_positional=verified)
            self.assertEqual(verified, {self.sha})
            adapter.verify_positional_engine(self.engine, self.sha.upper(), _verified_positional=verified)
            self.assertEqual(self.capability_calls, 1)
            with self.assertRaisesRegex(ValueError, 'qualified engine hash'):
                adapter.verify_positional_engine(self.engine, '0'*64, _verified_positional={'0'*64})
            self.engine.write_bytes(b'changed after qualification')
            with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                adapter.verify_positional_engine(self.engine, self.sha, _verified_positional=verified)
            self.assertEqual(self.capability_calls, 1)

    def test_failed_capabilities_do_not_enter_reuse_set(self):
        valid = self.capability
        for invalid in (b'{}', b'not-json', b'{"contract":"x","contract":"y"}', b' '*4097,
                        json.dumps(adapter.layout.CAPABILITY | {'required_item_count': 18}).encode()):
            verified = set()
            self.capability = invalid
            before = self.capability_calls
            with self.subTest(invalid=invalid[:50]), patch.object(adapter.subprocess, 'run', side_effect=self.launch):
                for _ in range(2):
                    with self.assertRaises(ValueError):
                        adapter.verify_positional_engine(self.engine, self.sha, _verified_positional=verified)
                    self.assertEqual(verified, set())
                self.assertEqual(self.capability_calls-before, 2)
                self.capability = valid
                adapter.verify_positional_engine(self.engine, self.sha, _verified_positional=verified)
                self.assertEqual(verified, {self.sha})

    def test_post_probe_tamper_or_deadline_never_memoises(self):
        verified = set()
        def tamper(*args, **kwargs):
            result = self.launch(*args, **kwargs)
            self.engine.write_bytes(b'changed during capability probe')
            return result
        with patch.object(adapter.subprocess, 'run', side_effect=tamper):
            with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                adapter.verify_positional_engine(self.engine, self.sha, _verified_positional=verified)
        self.assertEqual(verified, set())
        self.engine.write_bytes(b'fixed synthetic engine bytes')
        with patch.object(adapter.subprocess, 'run', side_effect=self.launch), \
                patch.object(adapter.time, 'monotonic', side_effect=[0, 2]):
            with self.assertRaisesRegex(ValueError, 'deadline exceeded'):
                adapter.verify_positional_engine(self.engine, self.sha, deadline=1, _verified_positional=verified)
        self.assertEqual(verified, set())
        with patch.object(adapter.subprocess, 'run') as launch:
            with self.assertRaisesRegex(ValueError, 'deadline exceeded'):
                adapter.verify_positional_engine(self.engine, self.sha, deadline=time.monotonic()-1,
                                                 _verified_positional={self.sha})
            launch.assert_not_called()

    def test_standalone_run_still_probes_every_cache_hit(self):
        with patch.object(adapter.subprocess, 'run', side_effect=self.launch):
            first = adapter.run(self.engine, self.sha, self.requests[0], cache=self.cache)
            second = adapter.run(self.engine, self.sha, self.requests[0], cache=self.cache)
        self.assertEqual((self.capability_calls, self.simulation_calls), (2, 1))
        self.assertEqual((first['cache_status'], second['cache_status']), ('miss', 'hit'))


class RealPositionalProbeTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_POSITIONAL_ARTIFACTS'), 'Explicit pinned native artifacts required')
    def test_real_compare_one_probe_per_cold_or_warm_batch(self):
        engine = Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS'])/'wowsimcli-positional.exe'
        fixture = json.loads((ROOT/'tests/fixtures/forever-hunter-bm.json').read_text())
        request = dict(raid=dict(parties=[dict(players=[fixture['player']])],
                                buffs=fixture['raidBuffs'], debuffs=fixture['debuffs']),
                       encounter=fixture['encounter'], type='SimTypeIndividual',
                       simOptions=dict(iterations=2, randomSeed='20261115'))
        request['raid']['parties'][0]['players'][0]['equipment']['preserveSlots'] = True
        other = deepcopy(request)
        other['raid']['parties'][0]['players'][0]['cooldowns'] = dict(hpPercentForDefensives=.5)
        with tempfile.TemporaryDirectory(prefix='native-probe-test-') as folder:
            cache = SimulationCache(Path(folder)/'cache')
            with patch.object(adapter.subprocess, 'run', wraps=subprocess.run) as launch:
                cold = adapter.compare(engine, adapter.layout.ENGINE_SHA256, [request, other], cache=cache)
                self.assertEqual(sum(c.args[0][1]=='azeroth-equipment-contract' for c in launch.call_args_list), 1)
                self.assertEqual(sum(c.args[0][1]=='sim' for c in launch.call_args_list), 2)
                warm = adapter.compare(engine, adapter.layout.ENGINE_SHA256, [request, other], cache=cache)
                self.assertEqual(sum(c.args[0][1]=='azeroth-equipment-contract' for c in launch.call_args_list), 2)
                self.assertEqual(sum(c.args[0][1]=='sim' for c in launch.call_args_list), 2)
            self.assertEqual([r['cache_status'] for r in cold['results']], ['miss', 'miss'])
            self.assertEqual([r['cache_status'] for r in warm['results']], ['hit', 'hit'])
            for before, after in zip(cold['results'], warm['results']):
                self.assertGreater(before['mean_dps'], 0)
                self.assertEqual(before['mean_dps'], after['mean_dps'])
                self.assertEqual(before['dps_standard_deviation'], after['dps_standard_deviation'])


if __name__ == '__main__':
    unittest.main()
