"""Combined fixed rewards use one saved bag model; alternatives are not additive."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_quest_turn_in_rewards import rewarded_sample


def sample():
    value = rewarded_sample()
    value['sections']['bags'] = {'0': {
        '1': {'item_id': 100, 'count': 19, 'link': 'PRIVATE_LINK', 'reported_stack_limit': 20},
        '2': {'item_id': 101, 'count': 3, 'link': 'PRIVATE_OTHER_LINK', 'reported_stack_limit': 20}}}
    value['sections']['bag_capacity'] = {
        '0': {'total_slots': 16, 'free_slots': 1, 'family_mask': 0},
        '1': {'total_slots': 16, 'free_slots': 10, 'family_mask': 32}}
    value['sections']['bag_capacity'].update({str(i): {'total_slots': 0, 'free_slots': 0} for i in range(2, 5)})
    return value


class QuestRewardBagSpaceTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_deduplicated_rewards_unknowns_and_model_bounds(self):
        root = Path(__file__).resolve().parents[1]; base = sample()
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store = SnapshotStore(Path(folder)); key = store.ingest(base)
            stranger = deepcopy(base); stranger['character']['name'] = 'PRIVATE_FOREIGN'
            foreign_key = SnapshotStore(Path(foreign)).ingest(stranger)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}; env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'inspect_snapshot')
                    def objects(value):
                        if isinstance(value, dict):
                            yield value
                            for child in value.values(): yield from objects(child)
                        elif isinstance(value, list):
                            for child in value: yield from objects(child)
                    schema = next(obj for obj in objects(route.inputSchema)
                                  if 'include_reward_bag_space' in obj.get('properties', {}))
                    self.assertFalse(schema['additionalProperties'])
                    self.assertEqual(schema['properties']['include_reward_bag_space']['type'], 'boolean')
                    self.assertFalse(schema['properties']['include_reward_bag_space']['default'])
                    async def call(value=None, snapshot=key, error=False, page=None, **extra):
                        if value is not None: snapshot = store.ingest(value)
                        if page is None: page = {'include_reward_review': True,
                            'include_reward_bag_space': True, 'ready_for_turn_in': True, **extra}
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': snapshot,
                            'section': 'quest_giver_references', 'quest_turn_in_review': page})
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply)); self.assertNotIn(foreign, str(reply))
                        return None if error else json.loads(reply.content[0].text)['data']
                    enabled = await call()
                    union = enabled['unique_quest_reward_review']; space = union['fixed_reward_bag_space']
                    self.assertEqual(union['quest_ids'], [1, 2, 7])
                    self.assertEqual([(r['item_id'], r['known_fixed_reward_quantity']) for r in space['items']], [(100, 3), (101, 1)])
                    self.assertEqual(space['recorded_general_free_slots'], 1)
                    self.assertEqual(space['all_fixed_rewards_estimated_new_stack_slots'], 2)
                    self.assertFalse(space['all_fixed_rewards_fit_recorded_general_slots'])
                    self.assertEqual(space['all_fixed_rewards_estimated_new_stack_slots_if_same_item_stacks_merge'], 1)
                    self.assertTrue(space['all_fixed_rewards_fit_recorded_general_slots_if_same_item_stacks_merge'])
                    self.assertTrue(space['all_fixed_reward_lists_observed'])
                    for flag in ['same_item_stack_merge_verified', 'placement_verified', 'choice_rewards_included',
                                 'quest_item_consumption_included', 'reward_delivery_verified', 'game_execution', 'live']:
                        self.assertFalse(space[flag])
                    for offset in [0, 1, 4294967295]:
                        self.assertEqual((await call(offset=offset, limit=1))['unique_quest_reward_review'], union)
                    # One quest can have several finishers; per-group space views must not be summed.
                    self.assertFalse(enabled['group_reward_views_are_additive'])
                    one = await call(quest_ids=[1, 2])
                    self.assertEqual(len(one['unique_quest_reward_review']['fixed_reward_bag_space']['items']), 1)
                    self.assertEqual(one['unique_quest_reward_review']['fixed_reward_bag_space']['estimated_new_stack_slots'], 1)
                    unknown = deepcopy(base); unknown['sections']['quests']['2'].pop('item_rewards')
                    partial = (await call(unknown))['unique_quest_reward_review']['fixed_reward_bag_space']
                    self.assertFalse(partial['all_fixed_reward_lists_observed'])
                    self.assertEqual(partial['unknown_fixed_reward_list_count'], 1)
                    self.assertEqual(partial['estimated_new_stack_slots'], 2)
                    self.assertIsNone(partial['all_fixed_rewards_estimated_new_stack_slots'])
                    self.assertIsNone(partial['all_fixed_rewards_fit_recorded_general_slots_if_same_item_stacks_merge'])
                    missing = deepcopy(base); missing['sections'].pop('bag_capacity')
                    observed = (await call(missing))['unique_quest_reward_review']['fixed_reward_bag_space']
                    self.assertIsNone(observed['recorded_general_free_slots'])
                    self.assertIsNone(observed['all_fixed_rewards_fit_recorded_general_slots'])
                    missing = deepcopy(base); missing['sections'].pop('bags')
                    self.assertIsNone((await call(missing))['unique_quest_reward_review']['fixed_reward_bag_space']['estimated_new_stack_slots'])
                    conflict = deepcopy(base)
                    conflict['sections']['bags']['0']['3'] = {'item_id': 100, 'count': 1,
                        'link': 'PRIVATE_CONFLICT', 'reported_stack_limit': 40}
                    self.assertIsNone((await call(conflict))['unique_quest_reward_review']['fixed_reward_bag_space']['estimated_new_stack_slots'])
                    empty = (await call(quest_ids=[3], ready_for_turn_in=False))['unique_quest_reward_review']['fixed_reward_bag_space']
                    self.assertEqual(empty['items'], []); self.assertEqual(empty['all_fixed_rewards_estimated_new_stack_slots'], 0)
                    old = await call(page={'include_reward_review': True})
                    self.assertNotIn('fixed_reward_bag_space', old['unique_quest_reward_review'])
                    self.assertEqual(old, await call(page={'include_reward_review': True, 'include_reward_bag_space': False}))
                    # Independent integer reference over two rewards and one captured stack.
                    cases = 0
                    for count in [1, 9, 19, 20]:
                        for first in [1, 19, 20]:
                            for second in [1, 2, 21]:
                                for free in [0, 1]:
                                    saved = deepcopy(base)
                                    saved['sections']['bags']['0']['1']['count'] = count
                                    saved['sections']['bag_capacity']['0']['free_slots'] = free
                                    saved['sections']['quests']['1']['item_rewards']['1']['quantity'] = first
                                    saved['sections']['quests']['2']['item_rewards']['1']['quantity'] = second
                                    actual = (await call(saved, quest_ids=[1, 2]))['unique_quest_reward_review']['fixed_reward_bag_space']
                                    slots = (first+second+19)//20
                                    merged = (max(0, first+second-(20-count))+19)//20
                                    self.assertEqual(actual['all_fixed_rewards_estimated_new_stack_slots'], slots)
                                    self.assertEqual(actual['all_fixed_rewards_estimated_new_stack_slots_if_same_item_stacks_merge'], merged)
                                    self.assertEqual(actual['all_fixed_rewards_fit_recorded_general_slots'], slots <= free)
                                    self.assertEqual(actual['all_fixed_rewards_fit_recorded_general_slots_if_same_item_stacks_merge'], merged <= free)
                                    cases += 1
                    self.assertEqual(cases, 72)
                    for bad in [[], [True, True], 'PRIVATE_MARKER', {'include_reward_bag_space': True},
                                {'include_reward_review': True, 'include_reward_bag_space': 1},
                                {'include_reward_review': True, 'include_reward_bag_space': 'PRIVATE_MARKER'},
                                {'PRIVATE_MARKER': True}]:
                        await call(page=bad, error=True)
                    await call(snapshot=foreign_key, error=True)
                    before = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await call()
                    self.assertEqual(before, {p: p.read_bytes() for p in before})
