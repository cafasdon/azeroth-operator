"""Local reward lookup joins the same saved gossip quest source, not eligibility."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class GossipRewardLookupTests(unittest.IsolatedAsyncioTestCase):
    async def test_source_correlated_reward_discovery_unknowns_and_completion(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            value = deepcopy(base)
            value['sections']['gossip_quests'] = {'coverage': 'visible_gossip_quest_lists',
                'available': {'166': {'is_complete': False}, '1475': {}},
                'active': {'166': {'is_complete': True}, '1445': {'is_complete': False}, '53': {}}}
            key = store.ingest(value)
            empty = deepcopy(base)
            empty['sections']['gossip_quests'] = {'coverage': 'visible_gossip_quest_lists', 'available': {}}
            empty_key = store.ingest(empty)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(snapshot=key, **query):
                        response = await session.call_tool('lookup_forever_catalogue', {'section': 'items',
                            'acquisition_snapshot_id': snapshot, **query})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    available = await lookup(gossip_quest_list='available')
                    ids = {row['id'] for row in available['records']}
                    self.assertTrue({1490, 2041, 2042} <= ids)
                    self.assertTrue(available['gossip_quest_filter']['list_observed'])
                    self.assertEqual(available['gossip_quest_filter']['observed_quest_count'], 2)
                    self.assertFalse(available['gossip_quest_filter']['eligibility_verified'])
                    complete = await lookup(gossip_quest_list='active', gossip_quest_is_complete=True)
                    self.assertTrue({2041, 2042} <= {row['id'] for row in complete['records']})
                    self.assertNotIn(744, {row['id'] for row in complete['records']})
                    incomplete = await lookup(gossip_quest_list='active', gossip_quest_is_complete=False)
                    self.assertIn(1490, {row['id'] for row in incomplete['records']})
                    self.assertNotIn(744, {row['id'] for row in incomplete['records']})
                    self.assertNotIn(2042, {row['id'] for row in incomplete['records']})
                    # Item 1490 has two quest sources. The filters must meet on one source.
                    mixed = await lookup(key='1490', reward_quest_id=1445, gossip_quest_list='available')
                    self.assertEqual(mixed['records'], [])
                    matched = await lookup(key='1490', reward_quest_id=1475, gossip_quest_list='available')
                    self.assertEqual([row['id'] for row in matched['records']], [1490])
                    sources = {row['quest_id']: row for row in matched['acquisition_progress']['sources'] if 'quest_id' in row}
                    self.assertFalse(sources[1445]['gossip_available_row_observed'])
                    self.assertTrue(sources[1445]['gossip_active_row_observed'])
                    self.assertFalse(sources[1445]['gossip_active_is_complete'])
                    self.assertTrue(sources[1475]['gossip_available_row_observed'])
                    self.assertFalse(sources[1475]['gossip_active_row_observed'])
                    self.assertIsNone(sources[1475]['gossip_active_is_complete'])
                    self.assertFalse(matched['acquisition_progress']['quests_available'])
                    self.assertFalse(sources[1475]['quest_observed'])
                    absent = await lookup(missing, gossip_quest_list='available')
                    self.assertEqual(absent['records'], [])
                    self.assertFalse(absent['gossip_quest_filter']['list_observed'])
                    self.assertIsNone(absent['gossip_quest_filter']['observed_quest_count'])
                    observed_empty = await lookup(empty_key, gossip_quest_list='available')
                    self.assertEqual(observed_empty['records'], [])
                    self.assertTrue(observed_empty['gossip_quest_filter']['list_observed'])
                    self.assertEqual(observed_empty['gossip_quest_filter']['observed_quest_count'], 0)
                    missing_active = await lookup(empty_key, gossip_quest_list='active')
                    self.assertFalse(missing_active['gossip_quest_filter']['list_observed'])
                    known_false = await lookup(gossip_quest_list='available', gossip_quest_is_complete=False)
                    self.assertIn(2042, {row['id'] for row in known_false['records']})
                    self.assertNotIn(1490, {row['id'] for row in known_false['records']})
                    legacy = await lookup(key='1490', reward_quest_id=1445)
                    self.assertEqual([row['id'] for row in legacy['records']], [1490])
                    self.assertNotIn('gossip_quest_filter', legacy)
                    for bad in [ {'section': 'items', 'gossip_quest_list': 'available'},
                                 {'section': 'items', 'key': '1490', 'gossip_quest_is_complete': False},
                                 {'section': 'consumables', 'key': '118', 'acquisition_snapshot_id': key, 'gossip_quest_list': 'available'},
                                 {'section': 'items', 'acquisition_snapshot_id': key, 'gossip_quest_list': 'all'},
                                 {'section': 'items', 'acquisition_snapshot_id': key, 'gossip_quest_list': 'active', 'gossip_quest_is_complete': 'false'}]:
                        response = await session.call_tool('lookup_forever_catalogue', bad)
                        self.assertTrue(response.isError)


if __name__ == '__main__':
    unittest.main()
