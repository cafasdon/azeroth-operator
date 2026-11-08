"""Saved visible gossip lists never imply acceptance or turn-in eligibility."""
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


class GossipQuestTests(unittest.IsolatedAsyncioTestCase):
    async def test_mcp_summary_inspection_unknowns_bounds_and_reader_parity(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        section = {'coverage': 'visible_gossip_quest_lists', 'available': {'10': {'repeatable': False}, '2': {}},
                   'active': {'20': {'is_complete': True}, '21': {'is_complete': False}, '22': {}}}
        cases = [None, section, {'coverage': section['coverage'], 'available': {}},
                 {'coverage': section['coverage'], 'active': {str(i): {'is_complete': True} for i in range(1, 129)}}]
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for value in cases:
                sample = deepcopy(base)
                if value is not None:
                    sample['sections']['gossip_quests'] = value
                valid.append((store.ingest(sample), value))
            invalid = [None, {}, {'coverage': section['coverage']}, {**section, 'coverage': 'all_quests'},
                       {**section, 'available': None}, {**section, 'available': []},
                       {**section, 'available': {'0': {}}}, {**section, 'available': {'01': {}}},
                       {**section, 'available': {'4294967296': {}}}, {**section, 'available': {'2': None}},
                       {**section, 'available': {str(i): {} for i in range(1, 130)}},
                       {**section, 'speaker': 'PRIVATE_GOSSIP'}]
            for field in ['is_complete', 'repeatable', 'is_trivial', 'is_ignored']:
                for bad in [None, 0, 'false', [], {}]:
                    invalid.append({**section, 'available': {'2': {field: bad}}})
            invalid.append({**section, 'available': {'2': {'title': 'PRIVATE_GOSSIP'}}})
            bad_ids = []
            for value in invalid:
                sample = deepcopy(base)
                sample['sections']['gossip_quests'] = value
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                sample = deepcopy(base)
                sample['client']['family'] = family
                sample['sections']['gossip_quests'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    for key, value in valid:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertFalse(response.isError)
                        summary = json.loads(response.content[0].text)['gossip_quest_observations']
                        if value is None:
                            self.assertIsNone(summary)
                            continue
                        self.assertFalse(summary['acceptance_eligibility_verified'])
                        self.assertFalse(summary['turn_in_eligibility_verified'])
                        for kind in ['available', 'active']:
                            rows = value.get(kind)
                            if rows is None:
                                self.assertIsNone(summary[kind])
                                continue
                            ids = sorted(map(int, rows))
                            complete = [i for i in ids if rows[str(i)].get('is_complete') is True]
                            self.assertEqual(summary[kind]['observed_quest_count'], len(ids))
                            self.assertEqual(summary[kind]['quest_ids'], ids[:50])
                            self.assertEqual(summary[kind]['quest_ids_truncated'], len(ids) > 50)
                            self.assertEqual(summary[kind]['reported_complete_quest_ids'], complete[:50])
                            self.assertEqual(summary[kind]['reported_complete_quest_count'], len(complete))
                            self.assertEqual(summary[kind]['reported_complete_truncated'], len(complete) > 50)
                            self.assertEqual(summary[kind]['quests_with_unknown_completion'], sum('is_complete' not in row for row in rows.values()))
                        inspected = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'gossip_quests'})
                        self.assertFalse(inspected.isError)
                        self.assertEqual(json.loads(inspected.content[0].text)['data'], value)
                    for key in bad_ids:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE_GOSSIP', str(response))


if __name__ == '__main__':
    unittest.main()
