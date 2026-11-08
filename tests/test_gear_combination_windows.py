"""Bounded coordinates preserve the exhaustive product without prefix walking."""
import asyncio
from contextlib import redirect_stdout
import io
from itertools import combinations, product
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.rank_forever_slot import combination_window, collect, main


class CombinationWindowTests(unittest.TestCase):
    def test_exhaustive_oracle_and_large_direct_offset(self):
        for lengths in product(range(1, 4), repeat=3):
            options = [[None] + [(slot, i) for i in range(1, length)]
                       for slot, length in enumerate(lengths)]
            for swaps in [None, 1, 2, 3, 17]:
                if swaps is None:
                    expected = list(product(*options))
                else:
                    groups = [row[1:] for row in options if len(row) > 1]
                    expected = [(None,)] + [row for size in range(1, min(swaps, len(groups)) + 1)
                                           for group in combinations(groups, size) for row in product(*group)]
                for limit in [1, 2, 7, 1024]:
                    joined = []
                    for offset in range(0, len(expected), limit):
                        total, window = combination_window(options, swaps, offset, limit)
                        self.assertEqual(total, len(expected))
                        actual = list(window)
                        self.assertEqual(actual, expected[offset:offset + limit])
                        joined.extend(actual)
                    self.assertEqual(joined, expected)
                    total, window = combination_window(options, swaps, len(expected) + 1, limit)
                    self.assertEqual(list(window), [])
        options = [[None, 1, 2, 3, 4] for _ in range(17)]
        total, window = combination_window(options, None, 5**17 - 1, 1)
        self.assertEqual(total, 762939453125)
        self.assertEqual(list(window), [(4,) * 17])
        for offset, limit in [(-1, 1), (True, 1), (0, 0), (0, True), (0, 1025)]:
            with self.assertRaises(ValueError):
                combination_window([[None]], None, offset, limit)

    def test_real_mcp_large_pool_cli_and_scope_binding(self):
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
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['bags'] = {'0': {str(i): {'item_id': item, 'count': 1,
                'link': f'item:{item}:0:0:0:0:0:0'} for i, item in enumerate(ids, 1)}}
            store = SnapshotStore(Path(folder))
            key = store.ingest(value)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            with patch.dict(os.environ, env):
                async def page(offset, **extra):
                    return await collect(binary, key, ['finger1', 'finger2'], errlog, collapse_identical=True,
                                         max_drafts=128, combination_offset=offset, combination_limit=64, **extra)
                first = asyncio.run(asyncio.wait_for(page(0), 45))
                second = asyncio.run(asyncio.wait_for(page(64), 45))
                a, b = first[-1]['combination_window'], second[-1]['combination_window']
                self.assertEqual(first[-1]['combinations_including_baseline'], 1089)
                self.assertEqual(a['next_offset'], 64)
                self.assertEqual(b['next_offset'], 128)
                self.assertEqual(a['scope_sha256'], b['scope_sha256'])
                self.assertFalse(a['covers_requested_combinations'])
                self.assertEqual(a['total_combinations_decimal'], '1089')
                last = asyncio.run(asyncio.wait_for(page(1088), 45))[-1]['combination_window']
                self.assertEqual(last['considered'], 1)
                self.assertIsNone(last['next_offset'])
                narrowed = asyncio.run(asyncio.wait_for(page(0, item_ids=ids[:2]), 45))[-1]['combination_window']
                self.assertNotEqual(a['scope_sha256'], narrowed['scope_sha256'])
                template, _, _ = inputs()
                template_path = Path(folder) / 'template.json'
                template_path.write_text(json.dumps(template))
                output = io.StringIO()
                argv = ['--mcp', str(binary), '--snapshot-id', key, '--slot', 'finger1', '--slot', 'finger2',
                        '--template', str(template_path), '--plan-only', '--collapse-identical-items',
                        '--combination-offset', '64', '--combination-limit', '64', '--max-drafts', '128']
                with redirect_stdout(output):
                    with patch('sys.argv', ['rank_forever_slot', *argv]):
                        main()
                result = json.loads(output.getvalue())
                self.assertEqual(result['gear_search']['combination_window']['scope_sha256'], a['scope_sha256'])
                self.assertFalse(result['best_in_slot_verified'])
                process = subprocess.run([sys.executable, '-m', 'tools.rank_forever_slot', *argv],
                                         cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(process.returncode, 0, process.stderr)
                launched = json.loads(process.stdout)
                self.assertEqual(launched['gear_search']['combination_window'], result['gear_search']['combination_window'])
                self.assertFalse(launched['best_in_slot_verified'])
                original_call = ClientSession.call_tool
                for invalid_source in [{}, None, {'sha256': None}, {'sha256': True},
                                       {'sha256': '0' * 63}, {'sha256': 'A' * 64}, 'changed']:
                    seen = 0
                    async def corrupt(session, name, *args, **kwargs):
                        nonlocal seen
                        reply = await original_call(session, name, *args, **kwargs)
                        if name == 'review_forever_owned_gear':
                            seen += 1
                            data = json.loads(reply.content[0].text)
                            if invalid_source == 'changed':
                                if seen > 1:
                                    data['catalogue_source']['sha256'] = '1' * 64
                            else:
                                data['catalogue_source'] = invalid_source
                            reply.content[0].text = json.dumps(data)
                        return reply
                    with patch('tools.rank_forever_slot.ClientSession.call_tool', corrupt):
                        try:
                            asyncio.run(page(0))
                        except Exception as error:
                            while isinstance(error, ExceptionGroup) and len(error.exceptions) == 1:
                                error = error.exceptions[0]
                            self.assertIsInstance(error, ValueError)
                            self.assertEqual(str(error), 'Local MCP returned inconsistent catalogue evidence')
                        else:
                            self.fail('Unknown catalogue provenance must reject a window')
                for kwargs in [{'combination_offset': -1}, {'combination_limit': 0},
                               {'combination_limit': 1025}, {'combination_offset': 1},
                               {'character': ('Fixture', 'Test'), 'combination_limit': 1}]:
                    with self.assertRaises(ValueError):
                        asyncio.run(collect(binary, key, ['finger1'], errlog, **kwargs))
